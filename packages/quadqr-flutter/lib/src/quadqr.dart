import 'dart:convert';
import 'dart:typed_data';
import 'models.dart';
import 'native_bindings.dart';
import 'options.dart';

class QuadQR {
  QuadQR._(); static final QuadQR instance=QuadQR._(); final NativeBindings _n=NativeBindings.instance;
  int _cameraMissStreak = 0;
  static const int _cameraRecoveryAfter = 3;
  static const int _cameraRecoveryEvery = 4;
  static const int _cameraWorkingMaxDimension = 960;
  Uint8List _binary(Map<String,dynamic> r)=>base64Decode((r['dataBase64']??'') as String);

  VersionInfo version()=>VersionInfo(_n.noArgs('quadqr_version_json'));
  VersionInfo getVersionInfo(int version,{EccLevel ecc=EccLevel.m,CellEncoding cellEncoding=CellEncoding.rgbw})=>VersionInfo(_n.versionOptions('quadqr_get_version_info_json',version,{'ecc':ecc.name.toUpperCase(),'cellEncoding':cellEncoding==CellEncoding.highDensity?'triangle16':'rgbw','highDensity':cellEncoding==CellEncoding.highDensity}));
  CapacityInfo capacity(int version,{EccLevel ecc=EccLevel.m,CellEncoding cellEncoding=CellEncoding.rgbw})=>CapacityInfo(_n.versionOptions('quadqr_capacity_json',version,{'ecc':ecc.name.toUpperCase(),'cellEncoding':cellEncoding==CellEncoding.highDensity?'triangle16':'rgbw','highDensity':cellEncoding==CellEncoding.highDensity}));
  CapacityPlan calculateCapacityPlan(Uint8List data,{EncodeOptions options=const EncodeOptions()})=>CapacityPlan(_n.bytesOptions('quadqr_calculate_capacity_plan_json',data,options.toMap()));
  EncodedQr encodeText(String text,{EncodeOptions options=const EncodeOptions()})=>EncodedQr(_n.twoStrings('quadqr_encode_text_json',text,options.toMap()));
  EncodedQr encodeBytes(Uint8List data,{EncodeOptions options=const EncodeOptions()})=>EncodedQr(_n.bytesOptions('quadqr_encode_bytes_json',data,options.toMap()));
  DecodedQr decodeMatrix(List<List<int>> matrix,{DecodeOptions options=const DecodeOptions()})=>DecodedQr(_n.twoStrings('quadqr_decode_matrix_json',jsonEncode(matrix),options.toMap()));
  Uint8List decodePayload(List<List<int>> matrix,{DecodeOptions options=const DecodeOptions()})=>_binary(_n.twoStrings('quadqr_decode_uint8_array_json',jsonEncode(matrix),options.toMap()));
  List<List<int>> rotateMatrix(List<List<int>> matrix,int quarterTurns)=>(_n.rotate(matrix,quarterTurns)['matrix'] as List).map((r)=>(r as List).cast<int>()).toList();
  int crc32(Uint8List data)=>_n.bytesOnly('quadqr_crc32_json',data)['crc32'] as int;
  Uint8List compress(Uint8List data,CompressionAlgorithm algorithm,{int level=6})=>_binary(_n.bytesOptions('quadqr_compress_${algorithm.name}_json',data,{'level':level,'compressionLevel':level,'quality':level}));
  Uint8List decompress(Uint8List data,CompressionAlgorithm algorithm,{int? expectedLength})=>_binary(_n.bytesOptions('quadqr_decompress_${algorithm.name}_json',data,{if(expectedLength!=null)'expectedLength':expectedLength}));
  RawKey generateRaw256Key()=>RawKey(_n.noArgs('quadqr_generate_raw_256_key_json'));
  SecurityInfo inspectSecurePayload(Uint8List payload)=>SecurityInfo(_n.bytesOnly('quadqr_inspect_secure_payload_json',payload));
  ApplicationPayload decrypt(DecodedQr decoded,SecurityOptions security)=>ApplicationPayload(_n.twoStrings('quadqr_decrypt_decoded_json',jsonEncode(decoded.nativeData),security.toMap()));
  SigningKeyPair generateSigningKeyPair()=>SigningKeyPair(_n.noArgs('quadqr_generate_signing_key_pair_json'));
  Uint8List deriveSigningKeyId(Uint8List publicKey,{int bytes=8})=>Uint8List.fromList((_n.bytesOptions('quadqr_derive_signing_key_id_json',publicKey,{'bytes':bytes})['keyId'] as List).cast<int>());
  Uint8List signingSeedFromPkcs8(Uint8List pkcs8)=>Uint8List.fromList((_n.bytesOnly('quadqr_signing_seed_from_pkcs8_json',pkcs8)['privateKeySeed'] as List).cast<int>());
  bool verifySignature(DecodedQr decoded,{Uint8List? trustedPublicKey,bool allowEmbeddedKey=false})=>_n.twoStrings('quadqr_verify_decoded_signature_json',jsonEncode(decoded.nativeData),{'allowEmbeddedKey':allowEmbeddedKey,if(trustedPublicKey!=null)'publicKey':trustedPublicKey.toList()})['verified']==true;
  ApplicationPayload decryptAndVerify(DecodedQr decoded,SecurityOptions security,{Uint8List? trustedPublicKey,bool allowEmbeddedKey=false})=>ApplicationPayload(_n.twoStrings('quadqr_decrypt_and_verify_decoded_json',jsonEncode(decoded.nativeData),{'security':security.toMap(),'allowEmbeddedKey':allowEmbeddedKey,if(trustedPublicKey!=null)'publicKey':trustedPublicKey.toList()}));
  Uint8List renderPng(EncodedQr code,{RenderOptions options=const RenderOptions(),Uint8List? logoImage})=>_binary(_n.renderPng(code.nativeData,options.toMap(),logoImage));
  String renderSvg(EncodedQr code,{RenderOptions options=const RenderOptions(),String? logoHref})=>_n.twoStrings('quadqr_render_svg_json',jsonEncode(code.nativeData),{...options.toMap(),if(logoHref!=null)'logoHref':logoHref})['svg'] as String;
  LogoSizeResult estimateSafeLogoSize(EncodedQr code,{RenderOptions options=const RenderOptions()})=>LogoSizeResult(_n.twoStrings('quadqr_estimate_safe_logo_size_json',jsonEncode(code.nativeData),options.toMap()));
  LogoSizeResult findMaxSafeLogoSize(EncodedQr code,Uint8List logo,{RenderOptions options=const RenderOptions(),double minSize=.05,double maxSize=.28,int iterations=7})=>LogoSizeResult(_n.findLogo(code.nativeData,logo,{...options.toMap(),'minSize':minSize,'maxSize':maxSize,'iterations':iterations}));
  PrintGuidance getPrintGuidance(EncodedQr code,{PrintOptions options=const PrintOptions()})=>PrintGuidance(_n.twoStrings('quadqr_get_print_guidance_json',jsonEncode(code.nativeData),options.toMap()));
  /// Scans a live RGBA camera frame.
  ///
  /// Normal frames use the cheap locator path. After three consecutive misses,
  /// every fourth attempt is allowed to use the full finder/rotation recovery
  /// stack. The full scanner working image is capped to 960px by default so
  /// high-resolution Android camera streams do not force full-resolution
  /// perspective work. Explicit [ScanOptions] values still win.
  ScanResult scanRgba(Uint8List rgba,int width,int height,{ScanOptions options=const ScanOptions()}) {
    final cameraOptions = Map<String,dynamic>.from(options.toMap());
    cameraOptions.putIfAbsent('workingMaxDimension', () => _cameraWorkingMaxDimension);
    final useDeepRecovery = _cameraMissStreak >= _cameraRecoveryAfter &&
        _cameraMissStreak % _cameraRecoveryEvery == _cameraRecoveryEvery - 1;
    if (!useDeepRecovery) {
      cameraOptions['finderRecovery'] = false;
      cameraOptions['rotationRecovery'] = false;
    }
    try {
      final result = ScanResult(_n.scan('quadqr_scan_rgba_json',rgba,width,height,cameraOptions));
      _cameraMissStreak = 0;
      return result;
    } catch (_) {
      _cameraMissStreak++;
      rethrow;
    }
  }

  /// Scans Android YUV_420_888 camera planes directly.
  ///
  /// This avoids the Dart-side YUV-to-RGBA conversion and lets the native
  /// bridge apply the 960px working cap while it converts the camera planes.
  ScanResult scanYuv420(
    Uint8List yPlane,
    Uint8List uPlane,
    Uint8List vPlane,
    int width,
    int height, {
    required int yRowStride,
    int yPixelStride = 1,
    required int uRowStride,
    required int uPixelStride,
    required int vRowStride,
    required int vPixelStride,
    ScanOptions options = const ScanOptions(),
  }) {
    final cameraOptions = Map<String, dynamic>.from(options.toMap());
    cameraOptions.putIfAbsent(
      'workingMaxDimension',
      () => _cameraWorkingMaxDimension,
    );
    final useDeepRecovery = _cameraMissStreak >= _cameraRecoveryAfter &&
        _cameraMissStreak % _cameraRecoveryEvery == _cameraRecoveryEvery - 1;
    if (!useDeepRecovery) {
      cameraOptions['finderRecovery'] = false;
      cameraOptions['rotationRecovery'] = false;
    }
    try {
      final result = ScanResult(
        _n.scanYuv420(
          yPlane,
          uPlane,
          vPlane,
          width,
          height,
          yRowStride: yRowStride,
          yPixelStride: yPixelStride,
          uRowStride: uRowStride,
          uPixelStride: uPixelStride,
          vRowStride: vRowStride,
          vPixelStride: vPixelStride,
          options: cameraOptions,
        ),
      );
      _cameraMissStreak = 0;
      return result;
    } catch (_) {
      _cameraMissStreak++;
      rethrow;
    }
  }

  /// Clears the adaptive live-camera miss counter, for example when reopening
  /// the camera or changing lenses.
  void resetCameraScanState() {
    _cameraMissStreak = 0;
  }

  ScanResult debugScanRgba(Uint8List rgba,int width,int height,{ScanOptions options=const ScanOptions()})=>ScanResult(_n.scan('quadqr_debug_scan_rgba_json',rgba,width,height,options.toMap()));
  ScanResult scanImage(Uint8List imageBytes,{ScanOptions options=const ScanOptions()})=>ScanResult(_n.bytesOptions('quadqr_scan_image_json',imageBytes,options.toMap()));
  RectifiedImage rectifyImage(Uint8List imageBytes,{ScanOptions options=const ScanOptions(),int moduleSize=8})=>RectifiedImage(_n.bytesOptions('quadqr_rectify_image_json',imageBytes,{...options.toMap(),'moduleSize':moduleSize}));
  Uint8List applyStressDistortion(Uint8List imageBytes,StressOptions options)=>_binary(_n.bytesOptions('quadqr_apply_stress_distortion_json',imageBytes,options.toMap()));
  int _unsignedCrc32(int crc32)=>crc32 & 0xffffffff;
  ReliabilityReport runImageStressTest(Uint8List imageBytes,int expectedCrc32,{ScanOptions options=const ScanOptions()})=>ReliabilityReport(_n.bytesOptions('quadqr_run_image_stress_test_json',imageBytes,{...options.toMap(),'expectedCrc32':_unsignedCrc32(expectedCrc32)}));
  ReliabilityReport runReliabilityLab(Uint8List imageBytes,int expectedCrc32,{ReliabilitySuite suite=ReliabilitySuite.quick,ScanOptions options=const ScanOptions()})=>ReliabilityReport(_n.bytesOptions('quadqr_run_reliability_lab_json',imageBytes,{...options.toMap(),'expectedCrc32':_unsignedCrc32(expectedCrc32),'suite':suite.name}));
  PerspectiveSweepResult runPerspectiveSweep(Uint8List imageBytes,int expectedCrc32,{PerspectiveAxis axis=PerspectiveAxis.yaw,List<double> angles=const [5,10,15,20,25,30,35,40],ScanOptions options=const ScanOptions()})=>PerspectiveSweepResult(_n.bytesOptions('quadqr_run_perspective_sweep_json',imageBytes,{...options.toMap(),'expectedCrc32':_unsignedCrc32(expectedCrc32),'axis':axis.name,'angles':angles}));
  ScanabilityAssessment assessScanability(EncodedQr code,{RenderOptions renderOptions=const RenderOptions(),ReliabilitySuite suite=ReliabilitySuite.quick,ScanOptions scanOptions=const ScanOptions()})=>ScanabilityAssessment(_n.twoStrings('quadqr_assess_scanability_json',jsonEncode(code.nativeData),{...renderOptions.toMap(),...scanOptions.toMap(),'suite':suite.name}));
}
