package io.github.akanshsirohi.quadqr;
import org.json.JSONObject;
public final class StressOptions {
    public final StressDistortion type; public final double severity,pitchDegrees,yawDegrees,rollDegrees;
    private StressOptions(Builder b){type=b.type;severity=b.severity;pitchDegrees=b.pitch;yawDegrees=b.yaw;rollDegrees=b.roll;}
    public static Builder builder(StressDistortion type){return new Builder(type);} public static final class Builder {final StressDistortion type;double severity=.5,pitch,yaw,roll;Builder(StressDistortion t){type=t;}public Builder severity(double v){severity=v;return this;}public Builder perspective(double pitch,double yaw,double roll){this.pitch=pitch;this.yaw=yaw;this.roll=roll;return this;}public StressOptions build(){return new StressOptions(this);}}
    JSONObject toJson(){try{return new JSONObject().put("type",type.value).put("severity",severity).put("perspective",new JSONObject().put("pitchDegrees",pitchDegrees).put("yawDegrees",yawDegrees).put("rollDegrees",rollDegrees));}catch(Exception e){throw new QuadQRException(e.getMessage(),e);}}
}
