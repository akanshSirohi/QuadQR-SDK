package io.github.akanshsirohi.quadqr;
import org.json.JSONObject;
public final class PrintOptions {
    public final int quietZone; public final double physicalSizeMm,dpi,minimumModuleMm;
    public PrintOptions(){this(4,35.0,300.0,0.40);} public PrintOptions(int quietZone,double physicalSizeMm,double dpi,double minimumModuleMm){this.quietZone=quietZone;this.physicalSizeMm=physicalSizeMm;this.dpi=dpi;this.minimumModuleMm=minimumModuleMm;}
    JSONObject toJson(){try{return new JSONObject().put("quietZone",quietZone).put("physicalSizeMm",physicalSizeMm).put("dpi",dpi).put("minimumModuleMm",minimumModuleMm);}catch(Exception e){throw new QuadQRException(e.getMessage(),e);}}
}
