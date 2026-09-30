package io.github.akanshsirohi.quadqr;
import org.json.JSONObject;
public final class CapacityInfo {
    private final JSONObject raw; CapacityInfo(JSONObject raw){this.raw=raw;} public int getVersion(){return raw.optInt("version");} public int getQuadQrBytes(){return raw.optInt("quadqrBytes");} public int getStandardQrBytes(){return raw.optInt("standardQrBytes");} public int getDifferenceBytes(){return raw.optInt("differenceBytes");} public double getRatio(){return raw.optDouble("ratio");} public double getGainPercent(){return raw.optDouble("gainPercent");} public int getSize(){return raw.optInt("size");} public boolean isHighDensity(){return raw.optBoolean("highDensity");}
}
