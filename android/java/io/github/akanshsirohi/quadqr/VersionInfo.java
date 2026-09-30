package io.github.akanshsirohi.quadqr;
import org.json.JSONObject;
public final class VersionInfo {
    private final JSONObject raw; VersionInfo(JSONObject raw){this.raw=raw;} public String getNativeBridgeVersion(){return raw.optString("nativeBridgeVersion",null);} public int getFormatVersion(){return raw.optInt("formatVersion");} public int getMinVersion(){return raw.optInt("minVersion",1);} public int getMaxVersion(){return raw.optInt("maxVersion",40);} public boolean isParallel(){return raw.optBoolean("parallel");}
    public int getVersion(){return raw.optInt("version");} public int getSize(){return raw.optInt("size");} public int getCapacityBytes(){return raw.optInt("capacityBytes");} public int getBitsPerDataCell(){return raw.optInt("bitsPerDataCell");} public int getDataCells(){return raw.optInt("dataCells");} public boolean isHighDensity(){return raw.optBoolean("highDensity");}
}
