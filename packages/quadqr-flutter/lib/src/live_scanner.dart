import 'dart:async';
import 'dart:isolate';
import 'dart:typed_data';

import 'models.dart';
import 'native_bindings.dart';
import 'options.dart';

/// Background live-camera scanner with a single pending-frame slot.
///
/// If frames arrive while native scanning is busy, older pending frames are
/// replaced and only the newest one is scanned next. On Android, prefer
/// [submitYuv420] for CameraImage YUV_420_888 frames so Dart never expands the
/// camera frame to a full RGBA buffer.
class QuadQRLiveScanner {
  QuadQRLiveScanner({this.options = const ScanOptions()});

  final ScanOptions options;
  final StreamController<ScanResult> _results =
      StreamController<ScanResult>.broadcast();

  Stream<ScanResult> get results => _results.stream;

  ReceivePort? _receivePort;
  Isolate? _isolate;
  SendPort? _workerPort;
  Map<String, dynamic>? _pending;
  bool _inFlight = false;
  bool _disposed = false;
  Completer<void>? _ready;

  Future<void> start() {
    if (_disposed) {
      return Future.error(StateError('QuadQRLiveScanner is disposed.'));
    }
    if (_workerPort != null) return Future.value();
    if (_ready != null) return _ready!.future;
    _ready = Completer<void>();
    _receivePort = ReceivePort();
    _receivePort!.listen(_handleWorkerMessage);
    Isolate.spawn(_quadQrLiveWorker, _receivePort!.sendPort).then((isolate) {
      if (_disposed) {
        isolate.kill(priority: Isolate.immediate);
      } else {
        _isolate = isolate;
      }
    }, onError: (Object error, StackTrace stack) {
      if (!(_ready?.isCompleted ?? true)) {
        _ready!.completeError(error, stack);
      }
    });
    return _ready!.future;
  }

  /// Submits an RGBA frame without queueing stale frames.
  ///
  /// Camera integrations should prefer [submitYuv420] on Android. RGBA remains
  /// useful for callers that already own a packed RGBA frame.
  void submitRgba(Uint8List rgba, int width, int height) {
    if (_disposed) {
      throw StateError('QuadQRLiveScanner is disposed.');
    }
    if (width <= 0 || height <= 0) {
      throw ArgumentError('width and height must be positive');
    }
    _pending = <String, dynamic>{
      'type': 'scanRgba',
      'rgba': TransferableTypedData.fromList([rgba]),
      'width': width,
      'height': height,
      'options': options.toMap(),
    };
    if (_workerPort == null) {
      unawaited(start());
      return;
    }
    _dispatchLatest();
  }

  /// Submits Android YUV_420_888 camera planes directly to the native scanner.
  ///
  /// The three planes are snapshotted into transferable buffers immediately,
  /// because camera plugins may reuse their backing buffers after the frame
  /// callback returns. Native code converts only the bounded working image,
  /// normally at most 960px on its longest side.
  void submitYuv420({
    required Uint8List yPlane,
    required Uint8List uPlane,
    required Uint8List vPlane,
    required int width,
    required int height,
    required int yRowStride,
    int yPixelStride = 1,
    required int uRowStride,
    required int uPixelStride,
    required int vRowStride,
    required int vPixelStride,
  }) {
    if (_disposed) {
      throw StateError('QuadQRLiveScanner is disposed.');
    }
    if (width <= 0 || height <= 0) {
      throw ArgumentError('width and height must be positive');
    }
    if (yRowStride <= 0 ||
        yPixelStride <= 0 ||
        uRowStride <= 0 ||
        uPixelStride <= 0 ||
        vRowStride <= 0 ||
        vPixelStride <= 0) {
      throw ArgumentError('YUV row/pixel strides must be positive');
    }
    _pending = <String, dynamic>{
      'type': 'scanYuv420',
      'y': TransferableTypedData.fromList([yPlane]),
      'u': TransferableTypedData.fromList([uPlane]),
      'v': TransferableTypedData.fromList([vPlane]),
      'width': width,
      'height': height,
      'yRowStride': yRowStride,
      'yPixelStride': yPixelStride,
      'uRowStride': uRowStride,
      'uPixelStride': uPixelStride,
      'vRowStride': vRowStride,
      'vPixelStride': vPixelStride,
      'options': options.toMap(),
    };
    if (_workerPort == null) {
      unawaited(start());
      return;
    }
    _dispatchLatest();
  }

  /// Drops any pending frame and resets the adaptive recovery cadence.
  void reset() {
    _pending = null;
    _workerPort?.send(const {'type': 'reset'});
  }

  void _handleWorkerMessage(dynamic message) {
    if (message is SendPort) {
      _workerPort = message;
      if (!(_ready?.isCompleted ?? true)) _ready!.complete();
      _dispatchLatest();
      return;
    }
    if (message is Map) {
      _inFlight = false;
      final raw = message['result'];
      if (raw is Map && !_results.isClosed) {
        _results.add(ScanResult(raw.cast<String, dynamic>()));
      }
      _dispatchLatest();
    }
  }

  void _dispatchLatest() {
    if (_disposed || _inFlight || _workerPort == null || _pending == null) {
      return;
    }
    final frame = _pending!;
    _pending = null;
    _inFlight = true;
    _workerPort!.send(frame);
  }

  Future<void> dispose() async {
    if (_disposed) return;
    _disposed = true;
    _pending = null;
    _workerPort?.send(const {'type': 'dispose'});
    _isolate?.kill(priority: Isolate.immediate);
    _receivePort?.close();
    await _results.close();
  }
}

void _quadQrLiveWorker(SendPort owner) {
  final receive = ReceivePort();
  owner.send(receive.sendPort);
  var missStreak = 0;

  receive.listen((dynamic message) {
    if (message is! Map) return;
    final type = message['type'];
    if (type == 'dispose') {
      receive.close();
      return;
    }
    if (type == 'reset') {
      missStreak = 0;
      return;
    }
    if (type != 'scanRgba' && type != 'scanYuv420') return;

    final scanOptions = Map<String, dynamic>.from(
      (message['options'] as Map).cast<String, dynamic>(),
    );
    scanOptions.putIfAbsent('workingMaxDimension', () => 960);

    final deep = missStreak >= 3 && missStreak % 4 == 3;
    if (!deep) {
      scanOptions['finderRecovery'] = false;
      scanOptions['rotationRecovery'] = false;
    }

    try {
      final Map<String, dynamic> result;
      if (type == 'scanYuv420') {
        final y = (message['y'] as TransferableTypedData)
            .materialize()
            .asUint8List();
        final u = (message['u'] as TransferableTypedData)
            .materialize()
            .asUint8List();
        final v = (message['v'] as TransferableTypedData)
            .materialize()
            .asUint8List();
        result = NativeBindings.instance.scanYuv420(
          y,
          u,
          v,
          message['width'] as int,
          message['height'] as int,
          yRowStride: message['yRowStride'] as int,
          yPixelStride: message['yPixelStride'] as int,
          uRowStride: message['uRowStride'] as int,
          uPixelStride: message['uPixelStride'] as int,
          vRowStride: message['vRowStride'] as int,
          vPixelStride: message['vPixelStride'] as int,
          options: scanOptions,
        );
      } else {
        final rgba = (message['rgba'] as TransferableTypedData)
            .materialize()
            .asUint8List();
        result = NativeBindings.instance.scan(
          'quadqr_scan_rgba_json',
          rgba,
          message['width'] as int,
          message['height'] as int,
          scanOptions,
        );
      }
      missStreak = 0;
      owner.send({'result': result});
    } catch (_) {
      // A miss is normal for a live camera. Keep it silent and let the main
      // isolate immediately dispatch the newest pending frame.
      missStreak++;
      owner.send(const {'miss': true});
    }
  });
}
