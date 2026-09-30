package io.github.akanshsirohi.quadqr;
import org.json.JSONObject;
public final class RawKey { private final byte[] key; private final String hex,base64; RawKey(JSONObject raw){key=JsonSupport.byteArray(raw.optJSONArray("key"));hex=raw.optString("hex");base64=raw.optString("base64");} public byte[] getBytes(){return key.clone();} public String getHex(){return hex;} public String getBase64(){return base64;} }
