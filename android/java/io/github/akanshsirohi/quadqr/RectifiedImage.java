package io.github.akanshsirohi.quadqr;
import org.json.JSONObject;
public final class RectifiedImage { private final int version;private final String path;private final byte[] png;RectifiedImage(JSONObject r){version=r.optInt("version");path=r.optString("path");JSONObject image=r.optJSONObject("image");png=image==null?new byte[0]:JsonSupport.bytes(image);}public int getVersion(){return version;}public String getPath(){return path;}public byte[] getPng(){return png.clone();} }
