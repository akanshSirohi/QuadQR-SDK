package io.github.akanshsirohi.quadqr;
import android.util.Base64; import org.json.JSONObject;
public class DecodedQr {
    final JSONObject raw; DecodedQr(JSONObject raw){this.raw=raw;} String nativeValue(){return raw.toString();}
    public String getText(){return raw.isNull("text")?null:raw.optString("text",null);} public byte[] getPayload(){String b64=raw.optString("payloadBase64","");return Base64.decode(b64,Base64.DEFAULT);}
    public int getVersion(){return raw.optInt("version");} public int getSize(){return raw.optInt("size");} public EccLevel getEccLevel(){try{return EccLevel.valueOf(raw.optString("eccLevel","M"));}catch(Exception e){return EccLevel.M;}}
    public boolean isHighDensity(){return raw.optBoolean("highDensity");} public boolean isSecure(){return raw.optBoolean("secure");} public boolean requiresDecryption(){return raw.optBoolean("requiresDecryption");}
    public boolean isSigned(){return raw.optBoolean("signed");} public boolean isSoftDecoded(){return raw.optBoolean("softDecoded");} public int getCorrectedSymbols(){return raw.optInt("correctedSymbols");} public int getErasureSymbols(){return raw.optInt("erasureSymbols");}
    public long getCrc32(){return raw.optLong("crc32") & 0xffffffffL;} public String getCompression(){return raw.optString("compression","none");} public String getSigningKeyId(){return raw.optString("signingKeyId",null);}
}
