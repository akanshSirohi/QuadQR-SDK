package io.github.akanshsirohi.quadqr;
import org.json.JSONObject;
public final class LogoOptions {
    final Double size,padding,radius; final Boolean autoSize,clearBackground; final String backgroundColor;
    private LogoOptions(Builder b){size=b.size;padding=b.padding;radius=b.radius;autoSize=b.autoSize;clearBackground=b.clearBackground;backgroundColor=b.backgroundColor;}
    public static Builder builder(){return new Builder();}
    public static final class Builder {Double size,padding,radius;Boolean autoSize,clearBackground;String backgroundColor;public Builder size(double v){size=v;autoSize=false;return this;}public Builder autoSize(){autoSize=true;return this;}public Builder padding(double v){padding=v;return this;}public Builder radius(double v){radius=v;return this;}public Builder clearBackground(boolean v){clearBackground=v;return this;}public Builder backgroundColor(String v){backgroundColor=v;return this;}public LogoOptions build(){return new LogoOptions(this);}}
    JSONObject toJson(){try{JSONObject o=new JSONObject();if(Boolean.TRUE.equals(autoSize))o.put("size","auto");else if(size!=null)o.put("size",size);if(padding!=null)o.put("padding",padding);if(radius!=null)o.put("radius",radius);if(clearBackground!=null)o.put("clearBackground",clearBackground);if(backgroundColor!=null)o.put("backgroundColor",backgroundColor);return o;}catch(Exception e){throw new QuadQRException(e.getMessage(),e);}}
}
