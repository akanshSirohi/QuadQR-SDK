package io.github.akanshsirohi.quadqr;
import java.util.*; import org.json.*;
public final class ScanResult extends DecodedQr {
    ScanResult(JSONObject raw){super(raw);}
    public boolean isPerspectiveCorrected(){return raw.optBoolean("perspectiveCorrected");}
    public boolean isColorCalibrated(){return raw.optBoolean("colorCalibrated");}
    public double getConfidence(){return raw.optDouble("confidence",0.0);}
    public String getScanPath(){return raw.optString("scanPath","");}
    public double getElapsedMs(){JSONObject d=raw.optJSONObject("diagnostics");return d==null?0.0:d.optDouble("elapsedMs",0.0);}
    public java.util.List<FinderPoint> getFinders(){JSONArray a=raw.optJSONArray("finders");java.util.List<FinderPoint> out=new java.util.ArrayList<>();if(a!=null)for(int i=0;i<a.length();i++){JSONObject v=a.optJSONObject(i);if(v!=null)out.add(new FinderPoint(v));}return java.util.Collections.unmodifiableList(out);}
}
