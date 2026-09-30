import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:typed_data';

import 'package:ffi/ffi.dart';

import 'models.dart';

typedef _NoArgsNative = Pointer<Utf8> Function();
typedef _NoArgsDart = Pointer<Utf8> Function();
typedef _TwoStringsNative = Pointer<Utf8> Function(
  Pointer<Utf8>,
  Pointer<Utf8>,
);
typedef _TwoStringsDart = Pointer<Utf8> Function(
  Pointer<Utf8>,
  Pointer<Utf8>,
);
typedef _BytesOptionsNative = Pointer<Utf8> Function(
  Pointer<Uint8>,
  IntPtr,
  Pointer<Utf8>,
);
typedef _BytesOptionsDart = Pointer<Utf8> Function(
  Pointer<Uint8>,
  int,
  Pointer<Utf8>,
);
typedef _BytesOnlyNative = Pointer<Utf8> Function(Pointer<Uint8>, IntPtr);
typedef _BytesOnlyDart = Pointer<Utf8> Function(Pointer<Uint8>, int);
typedef _VersionOptionsNative = Pointer<Utf8> Function(Uint8, Pointer<Utf8>);
typedef _VersionOptionsDart = Pointer<Utf8> Function(int, Pointer<Utf8>);
typedef _ScanNative = Pointer<Utf8> Function(
  Pointer<Uint8>,
  IntPtr,
  Uint32,
  Uint32,
  Pointer<Utf8>,
);
typedef _ScanDart = Pointer<Utf8> Function(
  Pointer<Uint8>,
  int,
  int,
  int,
  Pointer<Utf8>,
);
typedef _ScanYuv420Native = Pointer<Utf8> Function(
  Pointer<Uint8>, IntPtr,
  Pointer<Uint8>, IntPtr,
  Pointer<Uint8>, IntPtr,
  Uint32, Uint32,
  IntPtr, IntPtr,
  IntPtr, IntPtr,
  IntPtr, IntPtr,
  Pointer<Utf8>,
);
typedef _ScanYuv420Dart = Pointer<Utf8> Function(
  Pointer<Uint8>, int,
  Pointer<Uint8>, int,
  Pointer<Uint8>, int,
  int, int,
  int, int,
  int, int,
  int, int,
  Pointer<Utf8>,
);
typedef _RenderPngNative = Pointer<Utf8> Function(
  Pointer<Utf8>,
  Pointer<Utf8>,
  Pointer<Uint8>,
  IntPtr,
);
typedef _RenderPngDart = Pointer<Utf8> Function(
  Pointer<Utf8>,
  Pointer<Utf8>,
  Pointer<Uint8>,
  int,
);
typedef _FindLogoNative = Pointer<Utf8> Function(
  Pointer<Utf8>,
  Pointer<Uint8>,
  IntPtr,
  Pointer<Utf8>,
);
typedef _FindLogoDart = Pointer<Utf8> Function(
  Pointer<Utf8>,
  Pointer<Uint8>,
  int,
  Pointer<Utf8>,
);
typedef _RotateNative = Pointer<Utf8> Function(Pointer<Utf8>, Int32);
typedef _RotateDart = Pointer<Utf8> Function(Pointer<Utf8>, int);
typedef _FreeNative = Void Function(Pointer<Utf8>);
typedef _FreeDart = void Function(Pointer<Utf8>);

/// Internal FFI binding layer.
///
/// Flutter application code should use [QuadQR] from `quadqr.dart`, not this
/// class directly.
class NativeBindings {
  NativeBindings._()
      : _lib = _openLibrary() {
    _free = _lib.lookupFunction<_FreeNative, _FreeDart>('quadqr_string_free');
  }

  static final NativeBindings instance = NativeBindings._();

  final DynamicLibrary _lib;
  late final _FreeDart _free;

  static DynamicLibrary _openLibrary() {
    if (!Platform.isAndroid) {
      throw UnsupportedError(
        'quadqr_flutter currently supports Android. '
        'The Dart API is cross-platform ready, but native binaries for this '
        'platform have not been packaged yet.',
      );
    }
    return DynamicLibrary.open('libquadqr.so');
  }

  String _take(Pointer<Utf8> pointer) {
    if (pointer == nullptr) {
      throw const QuadQRException('The native QuadQR engine returned null.');
    }
    try {
      return pointer.toDartString();
    } finally {
      _free(pointer);
    }
  }

  Map<String, dynamic> _result(String raw) {
    final decoded = jsonDecode(raw);
    if (decoded is! Map) {
      throw const QuadQRException('The native QuadQR response was malformed.');
    }
    final map = decoded.cast<String, dynamic>();
    if (map['ok'] != true) {
      throw QuadQRException(
        (map['error'] ?? 'QuadQR operation failed').toString(),
      );
    }
    final result = map['result'];
    if (result is Map) return result.cast<String, dynamic>();
    return <String, dynamic>{'value': result};
  }

  Pointer<Uint8> _bytes(Uint8List data) {
    final pointer = calloc<Uint8>(data.isEmpty ? 1 : data.length);
    if (data.isNotEmpty) {
      pointer.asTypedList(data.length).setAll(0, data);
    }
    return pointer;
  }

  Map<String, dynamic> noArgs(String name) {
    final fn = _lib.lookupFunction<_NoArgsNative, _NoArgsDart>(name);
    return _result(_take(fn()));
  }

  Map<String, dynamic> twoStrings(
    String name,
    String first,
    Map<String, dynamic> second,
  ) {
    final fn = _lib.lookupFunction<_TwoStringsNative, _TwoStringsDart>(name);
    final firstPointer = first.toNativeUtf8();
    final secondPointer = jsonEncode(second).toNativeUtf8();
    try {
      return _result(_take(fn(firstPointer, secondPointer)));
    } finally {
      calloc.free(firstPointer);
      calloc.free(secondPointer);
    }
  }

  Map<String, dynamic> versionOptions(
    String name,
    int version,
    Map<String, dynamic> options,
  ) {
    final fn =
        _lib.lookupFunction<_VersionOptionsNative, _VersionOptionsDart>(name);
    final optionsPointer = jsonEncode(options).toNativeUtf8();
    try {
      return _result(_take(fn(version, optionsPointer)));
    } finally {
      calloc.free(optionsPointer);
    }
  }

  Map<String, dynamic> bytesOptions(
    String name,
    Uint8List data,
    Map<String, dynamic> options,
  ) {
    final fn =
        _lib.lookupFunction<_BytesOptionsNative, _BytesOptionsDart>(name);
    final dataPointer = _bytes(data);
    final optionsPointer = jsonEncode(options).toNativeUtf8();
    try {
      return _result(_take(fn(dataPointer, data.length, optionsPointer)));
    } finally {
      calloc.free(dataPointer);
      calloc.free(optionsPointer);
    }
  }

  Map<String, dynamic> bytesOnly(String name, Uint8List data) {
    final fn = _lib.lookupFunction<_BytesOnlyNative, _BytesOnlyDart>(name);
    final dataPointer = _bytes(data);
    try {
      return _result(_take(fn(dataPointer, data.length)));
    } finally {
      calloc.free(dataPointer);
    }
  }

  Map<String, dynamic> scan(
    String name,
    Uint8List data,
    int width,
    int height,
    Map<String, dynamic> options,
  ) {
    final fn = _lib.lookupFunction<_ScanNative, _ScanDart>(name);
    final dataPointer = _bytes(data);
    final optionsPointer = jsonEncode(options).toNativeUtf8();
    try {
      return _result(
        _take(fn(dataPointer, data.length, width, height, optionsPointer)),
      );
    } finally {
      calloc.free(dataPointer);
      calloc.free(optionsPointer);
    }
  }

  Map<String, dynamic> scanYuv420(
    Uint8List yPlane,
    Uint8List uPlane,
    Uint8List vPlane,
    int width,
    int height, {
    required int yRowStride,
    required int yPixelStride,
    required int uRowStride,
    required int uPixelStride,
    required int vRowStride,
    required int vPixelStride,
    required Map<String, dynamic> options,
  }) {
    final fn = _lib.lookupFunction<_ScanYuv420Native, _ScanYuv420Dart>(
      'quadqr_scan_yuv420_json',
    );
    final yPointer = _bytes(yPlane);
    final uPointer = _bytes(uPlane);
    final vPointer = _bytes(vPlane);
    final optionsPointer = jsonEncode(options).toNativeUtf8();
    try {
      return _result(
        _take(
          fn(
            yPointer,
            yPlane.length,
            uPointer,
            uPlane.length,
            vPointer,
            vPlane.length,
            width,
            height,
            yRowStride,
            yPixelStride,
            uRowStride,
            uPixelStride,
            vRowStride,
            vPixelStride,
            optionsPointer,
          ),
        ),
      );
    } finally {
      calloc.free(yPointer);
      calloc.free(uPointer);
      calloc.free(vPointer);
      calloc.free(optionsPointer);
    }
  }

  Map<String, dynamic> renderPng(
    Map<String, dynamic> code,
    Map<String, dynamic> options,
    Uint8List? logo,
  ) {
    final fn =
        _lib.lookupFunction<_RenderPngNative, _RenderPngDart>(
      'quadqr_render_png_json',
    );
    final codePointer = jsonEncode(code).toNativeUtf8();
    final optionsPointer = jsonEncode(options).toNativeUtf8();
    final logoBytes = logo ?? Uint8List(0);
    final logoPointer = _bytes(logoBytes);
    try {
      return _result(
        _take(
          fn(
            codePointer,
            optionsPointer,
            logoPointer,
            logoBytes.length,
          ),
        ),
      );
    } finally {
      calloc.free(codePointer);
      calloc.free(optionsPointer);
      calloc.free(logoPointer);
    }
  }

  Map<String, dynamic> findLogo(
    Map<String, dynamic> code,
    Uint8List logo,
    Map<String, dynamic> options,
  ) {
    final fn = _lib.lookupFunction<_FindLogoNative, _FindLogoDart>(
      'quadqr_find_max_safe_logo_size_json',
    );
    final codePointer = jsonEncode(code).toNativeUtf8();
    final optionsPointer = jsonEncode(options).toNativeUtf8();
    final logoPointer = _bytes(logo);
    try {
      return _result(
        _take(fn(codePointer, logoPointer, logo.length, optionsPointer)),
      );
    } finally {
      calloc.free(codePointer);
      calloc.free(optionsPointer);
      calloc.free(logoPointer);
    }
  }

  Map<String, dynamic> rotate(List<List<int>> matrix, int turns) {
    final fn = _lib.lookupFunction<_RotateNative, _RotateDart>(
      'quadqr_rotate_matrix_json',
    );
    final matrixPointer = jsonEncode(matrix).toNativeUtf8();
    try {
      return _result(_take(fn(matrixPointer, turns)));
    } finally {
      calloc.free(matrixPointer);
    }
  }
}
