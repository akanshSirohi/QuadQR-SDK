package io.github.akanshsirohi.quadqr;
import org.json.JSONObject;
public final class LogoSizeResult { private final JSONObject r; LogoSizeResult(JSONObject r){this.r=r;}public double getSafeSize(){return r.optDouble("safeSize");}public double getTestedMax(){return r.optDouble("testedMax");}public int getIterations(){return r.optInt("iterations");}public boolean isEmpirical(){return r.optBoolean("empirical");} }
