package io.github.akanshsirohi.quadqr;
import org.json.JSONObject;
public final class FinderPoint { private final double x,y,moduleSize,score; FinderPoint(JSONObject r){x=r.optDouble("x");y=r.optDouble("y");moduleSize=r.optDouble("moduleSize");score=r.optDouble("score");} public double getX(){return x;}public double getY(){return y;}public double getModuleSize(){return moduleSize;}public double getScore(){return score;} }
