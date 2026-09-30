package io.github.akanshsirohi.quadqr;

import org.json.JSONArray;
import org.json.JSONObject;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * High-level Android API for QuadQR. This is the class application code should use.
 */
public final class QuadQR {
    private static final QuadQR INSTANCE = new QuadQR();
    private static final int CAMERA_RECOVERY_AFTER = 3;
    private static final int CAMERA_RECOVERY_EVERY = 4;
    private static final int CAMERA_WORKING_MAX_DIMENSION = 960;
    private final AtomicInteger cameraMissStreak = new AtomicInteger();
    public static QuadQR getInstance(){ return INSTANCE; }
    private QuadQR() { /* triggers library loading on first native call */ }

    public VersionInfo version(){ return new VersionInfo(JsonSupport.result(QuadQRNative.nativeVersionJson())); }

    public VersionInfo getVersionInfo(int version, EccLevel ecc, CellEncoding encoding){
        try { JSONObject o=new JSONObject().put("ecc",ecc.name()).put("cellEncoding",encoding.value).put("highDensity",encoding==CellEncoding.HIGH_DENSITY); return new VersionInfo(JsonSupport.result(QuadQRNative.nativeGetVersionInfoJson(version,o.toString()))); }
        catch(Exception e){throw wrap(e);}
    }

    public CapacityInfo capacity(int version, EccLevel ecc, CellEncoding encoding){
        try { JSONObject o=new JSONObject().put("ecc",ecc.name()).put("cellEncoding",encoding.value).put("highDensity",encoding==CellEncoding.HIGH_DENSITY); return new CapacityInfo(JsonSupport.result(QuadQRNative.nativeCapacityJson(version,o.toString()))); }
        catch(Exception e){throw wrap(e);}
    }

    public CapacityPlan calculateCapacityPlan(byte[] data, EncodeOptions options){ return new CapacityPlan(JsonSupport.result(QuadQRNative.nativeCalculateCapacityPlanJson(data, json(options)))); }
    public EncodedQr encodeText(String text){ return encodeText(text, EncodeOptions.defaults()); }
    public EncodedQr encodeText(String text, EncodeOptions options){ return new EncodedQr(JsonSupport.result(QuadQRNative.nativeEncodeTextJson(text, json(options)))); }
    public EncodedQr encodeBytes(byte[] data){ return encodeBytes(data, EncodeOptions.defaults()); }
    public EncodedQr encodeBytes(byte[] data, EncodeOptions options){ return new EncodedQr(JsonSupport.result(QuadQRNative.nativeEncodeBytesJson(data, json(options)))); }

    public DecodedQr decodeMatrix(int[][] matrix){ return decodeMatrix(matrix, DecodeOptions.defaults()); }
    public DecodedQr decodeMatrix(int[][] matrix, DecodeOptions options){ return new DecodedQr(JsonSupport.result(QuadQRNative.nativeDecodeMatrixJson(JsonSupport.matrix(matrix).toString(), json(options)))); }
    public byte[] decodePayload(int[][] matrix, DecodeOptions options){ return JsonSupport.bytes(JsonSupport.result(QuadQRNative.nativeDecodeUint8ArrayJson(JsonSupport.matrix(matrix).toString(), json(options)))); }
    public int[][] rotateMatrix(int[][] matrix, int quarterTurns){ return JsonSupport.matrix(JsonSupport.result(QuadQRNative.nativeRotateMatrixJson(JsonSupport.matrix(matrix).toString(),quarterTurns)).optJSONArray("matrix")); }
    public long crc32(byte[] data){ return JsonSupport.result(QuadQRNative.nativeCrc32Json(data)).optLong("crc32") & 0xffffffffL; }

    public byte[] compress(byte[] data, CompressionAlgorithm algorithm, int level){
        try {String o=new JSONObject().put("level",level).put("compressionLevel",level).put("quality",level).toString(); switch(algorithm){case LZ:return JsonSupport.bytes(JsonSupport.result(QuadQRNative.nativeCompressLzJson(data,o)));case DEFLATE:return JsonSupport.bytes(JsonSupport.result(QuadQRNative.nativeCompressDeflateJson(data,o)));default:return JsonSupport.bytes(JsonSupport.result(QuadQRNative.nativeCompressBrotliJson(data,o)));}}
        catch(Exception e){throw wrap(e);}
    }
    public byte[] decompress(byte[] data, CompressionAlgorithm algorithm){ return decompress(data,algorithm,null); }
    public byte[] decompress(byte[] data, CompressionAlgorithm algorithm, Integer expectedLength){ try{String o=expectedLength==null?"{}":new JSONObject().put("expectedLength",expectedLength).toString(); switch(algorithm){case LZ:return JsonSupport.bytes(JsonSupport.result(QuadQRNative.nativeDecompressLzJson(data,o)));case DEFLATE:return JsonSupport.bytes(JsonSupport.result(QuadQRNative.nativeDecompressDeflateJson(data,o)));default:return JsonSupport.bytes(JsonSupport.result(QuadQRNative.nativeDecompressBrotliJson(data,o)));}}catch(Exception e){throw wrap(e);} }

    public RawKey generateRaw256Key(){ return new RawKey(JsonSupport.result(QuadQRNative.nativeGenerateRaw256KeyJson())); }
    public SecurityInfo inspectSecurePayload(byte[] securePayload){ return new SecurityInfo(JsonSupport.result(QuadQRNative.nativeInspectSecurePayloadJson(securePayload))); }
    public ApplicationPayload decrypt(DecodedQr decoded, SecurityOptions security){ return new ApplicationPayload(JsonSupport.result(QuadQRNative.nativeDecryptDecodedJson(decoded.nativeValue(),security.toJson().toString()))); }
    public SigningKeyPair generateSigningKeyPair(){ return new SigningKeyPair(JsonSupport.result(QuadQRNative.nativeGenerateSigningKeyPairJson())); }
    public byte[] deriveSigningKeyId(byte[] publicKey, int bytes){ try{return JsonSupport.byteArray(JsonSupport.result(QuadQRNative.nativeDeriveSigningKeyIdJson(publicKey,new JSONObject().put("bytes",bytes).toString())).optJSONArray("keyId"));}catch(Exception e){throw wrap(e);} }
    public byte[] signingSeedFromPkcs8(byte[] pkcs8){ return JsonSupport.byteArray(JsonSupport.result(QuadQRNative.nativeSigningSeedFromPkcs8Json(pkcs8)).optJSONArray("privateKeySeed")); }
    public boolean verifySignature(DecodedQr decoded, byte[] trustedPublicKey, boolean allowEmbeddedKey){ try{JSONObject o=new JSONObject().put("allowEmbeddedKey",allowEmbeddedKey);if(trustedPublicKey!=null)o.put("publicKey",JsonSupport.bytesArray(trustedPublicKey));return JsonSupport.result(QuadQRNative.nativeVerifyDecodedSignatureJson(decoded.nativeValue(),o.toString())).optBoolean("verified");}catch(Exception e){throw wrap(e);} }
    public ApplicationPayload decryptAndVerify(DecodedQr decoded, SecurityOptions security, byte[] trustedPublicKey, boolean allowEmbeddedKey){ try{JSONObject o=new JSONObject().put("security",security.toJson()).put("allowEmbeddedKey",allowEmbeddedKey);if(trustedPublicKey!=null)o.put("publicKey",JsonSupport.bytesArray(trustedPublicKey));return new ApplicationPayload(JsonSupport.result(QuadQRNative.nativeDecryptAndVerifyDecodedJson(decoded.nativeValue(),o.toString())));}catch(Exception e){throw wrap(e);} }

    public byte[] renderPng(EncodedQr code){ return renderPng(code,RenderOptions.defaults(),null); }
    public byte[] renderPng(EncodedQr code, RenderOptions options){ return renderPng(code,options,null); }
    public byte[] renderPng(EncodedQr code, RenderOptions options, byte[] logoImage){ return JsonSupport.bytes(JsonSupport.result(QuadQRNative.nativeRenderPngJson(code.nativeValue(),json(options),logoImage==null?new byte[0]:logoImage))); }
    public String renderSvg(EncodedQr code, RenderOptions options){ return renderSvg(code,options,null); }
    public String renderSvg(EncodedQr code, RenderOptions options, String logoHref){ try{JSONObject o=new JSONObject(json(options));if(logoHref!=null)o.put("logoHref",logoHref);return JsonSupport.result(QuadQRNative.nativeRenderSvgJson(code.nativeValue(),o.toString())).optString("svg");}catch(Exception e){throw wrap(e);} }
    public LogoSizeResult estimateSafeLogoSize(EncodedQr code, RenderOptions options){ return new LogoSizeResult(JsonSupport.result(QuadQRNative.nativeEstimateSafeLogoSizeJson(code.nativeValue(),json(options)))); }
    public LogoSizeResult findMaxSafeLogoSize(EncodedQr code, byte[] logoImage, RenderOptions options, double minSize, double maxSize, int iterations){ try{JSONObject o=new JSONObject(json(options)).put("minSize",minSize).put("maxSize",maxSize).put("iterations",iterations);return new LogoSizeResult(JsonSupport.result(QuadQRNative.nativeFindMaxSafeLogoSizeJson(code.nativeValue(),logoImage,o.toString())));}catch(Exception e){throw wrap(e);} }
    public PrintGuidance getPrintGuidance(EncodedQr code, PrintOptions options){ return new PrintGuidance(JsonSupport.result(QuadQRNative.nativeGetPrintGuidanceJson(code.nativeValue(),options.toJson().toString()))); }

    public ScanResult scanRgba(byte[] rgba, int width, int height){ return scanRgba(rgba,width,height,ScanOptions.defaults()); }
    /**
     * Live-camera RGBA scan. Normal frames use the cheap locator path; after
     * repeated misses, periodic frames use the full recovery stack. The full
     * working image is capped to 960px by default to bound Android camera cost.
     */
    public ScanResult scanRgba(byte[] rgba, int width, int height, ScanOptions options){
        int misses = cameraMissStreak.get();
        boolean useDeepRecovery = misses >= CAMERA_RECOVERY_AFTER
            && misses % CAMERA_RECOVERY_EVERY == CAMERA_RECOVERY_EVERY - 1;
        try {
            JSONObject o=(options==null?ScanOptions.defaults():options).toJson();
            if(!o.has("workingMaxDimension"))o.put("workingMaxDimension",CAMERA_WORKING_MAX_DIMENSION);
            if(!useDeepRecovery){
                o.put("finderRecovery",false);
                o.put("rotationRecovery",false);
            }
            ScanResult result=new ScanResult(JsonSupport.result(QuadQRNative.nativeScanRgbaJson(rgba,width,height,o.toString())));
            cameraMissStreak.set(0);
            return result;
        } catch(Exception e){
            cameraMissStreak.incrementAndGet();
            throw wrap(e);
        }
    }
    /**
     * Scans Android YUV_420_888 camera planes directly. This avoids converting
     * a full camera frame to RGBA in application code. The native bridge also
     * applies the live-camera working-size cap while converting YUV.
     */
    public ScanResult scanYuv420(
        byte[] yPlane, byte[] uPlane, byte[] vPlane,
        int width, int height,
        int yRowStride, int yPixelStride,
        int uRowStride, int uPixelStride,
        int vRowStride, int vPixelStride
    ){
        return scanYuv420(
            yPlane,uPlane,vPlane,width,height,
            yRowStride,yPixelStride,uRowStride,uPixelStride,vRowStride,vPixelStride,
            ScanOptions.defaults()
        );
    }

    public ScanResult scanYuv420(
        byte[] yPlane, byte[] uPlane, byte[] vPlane,
        int width, int height,
        int yRowStride, int yPixelStride,
        int uRowStride, int uPixelStride,
        int vRowStride, int vPixelStride,
        ScanOptions options
    ){
        int misses = cameraMissStreak.get();
        boolean useDeepRecovery = misses >= CAMERA_RECOVERY_AFTER
            && misses % CAMERA_RECOVERY_EVERY == CAMERA_RECOVERY_EVERY - 1;
        try {
            JSONObject o=(options==null?ScanOptions.defaults():options).toJson();
            if(!o.has("workingMaxDimension"))o.put("workingMaxDimension",CAMERA_WORKING_MAX_DIMENSION);
            if(!useDeepRecovery){
                o.put("finderRecovery",false);
                o.put("rotationRecovery",false);
            }
            ScanResult result=new ScanResult(JsonSupport.result(QuadQRNative.nativeScanYuv420Json(
                yPlane,uPlane,vPlane,width,height,
                yRowStride,yPixelStride,uRowStride,uPixelStride,vRowStride,vPixelStride,
                o.toString()
            )));
            cameraMissStreak.set(0);
            return result;
        } catch(Exception e){
            cameraMissStreak.incrementAndGet();
            throw wrap(e);
        }
    }

    public void resetCameraScanState(){ cameraMissStreak.set(0); }
    public ScanResult debugScanRgba(byte[] rgba, int width, int height, ScanOptions options){ return new ScanResult(JsonSupport.result(QuadQRNative.nativeDebugScanRgbaJson(rgba,width,height,json(options)))); }
    public ScanResult scanImage(byte[] imageBytes){ return scanImage(imageBytes,ScanOptions.defaults()); }
    public ScanResult scanImage(byte[] imageBytes, ScanOptions options){ return new ScanResult(JsonSupport.result(QuadQRNative.nativeScanImageJson(imageBytes,json(options)))); }
    public RectifiedImage rectifyImage(byte[] imageBytes, ScanOptions options, int moduleSize){ try{JSONObject o=new JSONObject(json(options)).put("moduleSize",moduleSize);return new RectifiedImage(JsonSupport.result(QuadQRNative.nativeRectifyImageJson(imageBytes,o.toString())));}catch(Exception e){throw wrap(e);} }

    public byte[] applyStressDistortion(byte[] imageBytes, StressOptions options){ return JsonSupport.bytes(JsonSupport.result(QuadQRNative.nativeApplyStressDistortionJson(imageBytes,options.toJson().toString()))); }
    public ReliabilityReport runImageStressTest(byte[] imageBytes, long expectedCrc32, ScanOptions scanOptions){ return new ReliabilityReport(JsonSupport.result(QuadQRNative.nativeRunImageStressTestJson(imageBytes,withExpected(scanOptions,expectedCrc32).toString()))); }
    public ReliabilityReport runReliabilityLab(byte[] imageBytes, long expectedCrc32, ReliabilitySuite suite, ScanOptions scanOptions){ try{JSONObject o=withExpected(scanOptions,expectedCrc32).put("suite",suite.value);return new ReliabilityReport(JsonSupport.result(QuadQRNative.nativeRunReliabilityLabJson(imageBytes,o.toString())));}catch(Exception e){throw wrap(e);} }
    public PerspectiveSweepResult runPerspectiveSweep(byte[] imageBytes, long expectedCrc32, PerspectiveAxis axis, double[] angles, ScanOptions scanOptions){ try{JSONArray a=new JSONArray();for(double v:angles)a.put(v);JSONObject o=withExpected(scanOptions,expectedCrc32).put("axis",axis.value).put("angles",a);return new PerspectiveSweepResult(JsonSupport.result(QuadQRNative.nativeRunPerspectiveSweepJson(imageBytes,o.toString())));}catch(Exception e){throw wrap(e);} }
    public ScanabilityAssessment assessScanability(EncodedQr code, RenderOptions renderOptions, ReliabilitySuite suite, ScanOptions scanOptions){ try{JSONObject o=new JSONObject(json(renderOptions));JsonSupport.merge(o,scanOptions.toJson());o.put("suite",suite.value);return new ScanabilityAssessment(JsonSupport.result(QuadQRNative.nativeAssessScanabilityJson(code.nativeValue(),o.toString())));}catch(Exception e){throw wrap(e);} }

    private static String json(EncodeOptions o){return (o==null?EncodeOptions.defaults():o).toJson().toString();}
    private static String json(DecodeOptions o){return (o==null?DecodeOptions.defaults():o).toJson().toString();}
    private static String json(ScanOptions o){return (o==null?ScanOptions.defaults():o).toJson().toString();}
    private static String json(RenderOptions o){return (o==null?RenderOptions.defaults():o).toJson().toString();}
    private static JSONObject withExpected(ScanOptions o,long crc){try{return (o==null?ScanOptions.defaults():o).toJson().put("expectedCrc32",crc & 0xffffffffL);}catch(Exception e){throw wrap(e);}}
    private static QuadQRException wrap(Exception e){return e instanceof QuadQRException?(QuadQRException)e:new QuadQRException(e.getMessage(),e);}
}
