package io.github.akanshsirohi.quadqr;

/** Internal native entry points. Developers should use {@link QuadQR}. */
final class QuadQRNative {
    static { System.loadLibrary("quadqr"); }
    private QuadQRNative() {}

    static native String nativeVersionJson();
    static native String nativeGetVersionInfoJson(int version, String optionsJson);
    static native String nativeCapacityJson(int version, String optionsJson);
    static native String nativeCalculateCapacityPlanJson(byte[] data, String optionsJson);
    static native String nativeEncodeTextJson(String text, String optionsJson);
    static native String nativeEncodeBytesJson(byte[] data, String optionsJson);
    static native String nativeDecodeMatrixJson(String matrixJson, String optionsJson);
    static native String nativeDecodeUint8ArrayJson(String matrixJson, String optionsJson);
    static native String nativeRotateMatrixJson(String matrixJson, int quarterTurns);
    static native String nativeCrc32Json(byte[] data);
    static native String nativeCompressLzJson(byte[] data, String optionsJson);
    static native String nativeDecompressLzJson(byte[] data, String optionsJson);
    static native String nativeCompressDeflateJson(byte[] data, String optionsJson);
    static native String nativeDecompressDeflateJson(byte[] data, String optionsJson);
    static native String nativeCompressBrotliJson(byte[] data, String optionsJson);
    static native String nativeDecompressBrotliJson(byte[] data, String optionsJson);
    static native String nativeGenerateRaw256KeyJson();
    static native String nativeInspectSecurePayloadJson(byte[] securePayload);
    static native String nativeDecryptDecodedJson(String decodedJson, String securityJson);
    static native String nativeGenerateSigningKeyPairJson();
    static native String nativeDeriveSigningKeyIdJson(byte[] publicKey, String optionsJson);
    static native String nativeSigningSeedFromPkcs8Json(byte[] pkcs8);
    static native String nativeVerifyDecodedSignatureJson(String decodedJson, String optionsJson);
    static native String nativeDecryptAndVerifyDecodedJson(String decodedJson, String optionsJson);
    static native String nativeRenderPngJson(String codeJson, String optionsJson, byte[] logoImageBytes);
    static native String nativeRenderSvgJson(String codeJson, String optionsJson);
    static native String nativeEstimateSafeLogoSizeJson(String codeJson, String optionsJson);
    static native String nativeFindMaxSafeLogoSizeJson(String codeJson, byte[] logoImageBytes, String optionsJson);
    static native String nativeGetPrintGuidanceJson(String codeJson, String optionsJson);
    static native String nativeScanRgbaJson(byte[] rgba, int width, int height, String optionsJson);
    static native String nativeScanYuv420Json(
        byte[] yPlane, byte[] uPlane, byte[] vPlane,
        int width, int height,
        int yRowStride, int yPixelStride,
        int uRowStride, int uPixelStride,
        int vRowStride, int vPixelStride,
        String optionsJson
    );
    static native String nativeDebugScanRgbaJson(byte[] rgba, int width, int height, String optionsJson);
    static native String nativeScanImageJson(byte[] encodedImageBytes, String optionsJson);
    static native String nativeRectifyImageJson(byte[] encodedImageBytes, String optionsJson);
    static native String nativeApplyStressDistortionJson(byte[] encodedImageBytes, String optionsJson);
    static native String nativeRunImageStressTestJson(byte[] encodedImageBytes, String optionsJson);
    static native String nativeRunReliabilityLabJson(byte[] encodedImageBytes, String optionsJson);
    static native String nativeRunPerspectiveSweepJson(byte[] encodedImageBytes, String optionsJson);
    static native String nativeAssessScanabilityJson(String codeJson, String optionsJson);
}
