package io.github.akanshsirohi.quadqr;
import org.json.JSONObject;
public final class EncodedQr {
    final JSONObject raw; EncodedQr(JSONObject raw){this.raw=raw;}
    String nativeValue(){return raw.toString();}
    public int getVersion(){return raw.optInt("version");} public int getSize(){return raw.optInt("size");} public int[][] getMatrix(){return JsonSupport.matrix(raw.optJSONArray("matrix"));}
    public EccLevel getEccLevel(){try{return EccLevel.valueOf(raw.optString("eccLevel","M"));}catch(Exception e){return EccLevel.M;}}
    public boolean isHighDensity(){return raw.optBoolean("highDensity");} public String getCellEncoding(){return raw.optString("cellEncoding");}
    public int getPayloadBytes(){return raw.optInt("payloadBytes");} public int getCapacityBytes(){return raw.optInt("capacityBytes");} public double getUtilization(){return raw.optDouble("utilization");}
    public long getCrc32(){return raw.optLong("crc32") & 0xffffffffL;} public boolean isCompressed(){return raw.optBoolean("compressed");} public String getCompression(){return raw.optString("compression","none");}
    public boolean isSecure(){return raw.optBoolean("secure");} public boolean isSigned(){return raw.optBoolean("signed");} public String getSigningKeyId(){return raw.optString("signingKeyId",null);}
}
