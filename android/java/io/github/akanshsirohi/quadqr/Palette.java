package io.github.akanshsirohi.quadqr;
import org.json.JSONObject;
public final class Palette {
    final String black,white,red,green,blue;
    private Palette(Builder b){black=b.black;white=b.white;red=b.red;green=b.green;blue=b.blue;}
    public static Builder builder(){return new Builder();}
    public static final class Builder {String black,white,red,green,blue; public Builder black(String v){black=v;return this;}public Builder white(String v){white=v;return this;}public Builder red(String v){red=v;return this;}public Builder green(String v){green=v;return this;}public Builder blue(String v){blue=v;return this;}public Palette build(){return new Palette(this);}}
    JSONObject toJson(){try{JSONObject o=new JSONObject();if(black!=null)o.put("black",black);if(white!=null)o.put("white",white);if(red!=null)o.put("red",red);if(green!=null)o.put("green",green);if(blue!=null)o.put("blue",blue);return o;}catch(Exception e){throw new QuadQRException(e.getMessage(),e);}}
}
