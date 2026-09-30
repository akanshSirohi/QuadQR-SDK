package io.github.akanshsirohi.quadqr;
import android.util.Base64; import org.json.JSONObject;
public final class ApplicationPayload {
    private final JSONObject raw; ApplicationPayload(JSONObject raw){this.raw=raw;} public String getText(){return raw.isNull("text")?null:raw.optString("text",null);} public byte[] getPayload(){return Base64.decode(raw.optString("payloadBase64",""),Base64.DEFAULT);} public boolean isCompressed(){return raw.optBoolean("compressed");} public String getCompression(){return raw.optString("compression","none");} public boolean isSigned(){return raw.optBoolean("signed");} public boolean isVerified(){return raw.optBoolean("verified");} public boolean isDecrypted(){return raw.optBoolean("decrypted");} public String getSigningKeyId(){return raw.optString("signingKeyId",null);}
}
