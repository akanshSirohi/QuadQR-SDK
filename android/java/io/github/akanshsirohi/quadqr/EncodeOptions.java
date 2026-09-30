package io.github.akanshsirohi.quadqr;

import org.json.JSONObject;

public final class EncodeOptions {
    final EccLevel ecc; final CellEncoding encoding; final CompressionMode compression; final Integer compressionLevel;
    final Integer version,minVersion,maxVersion; final SecurityOptions security; final SigningOptions signing;
    private EncodeOptions(Builder b){ecc=b.ecc;encoding=b.encoding;compression=b.compression;compressionLevel=b.compressionLevel;version=b.version;minVersion=b.minVersion;maxVersion=b.maxVersion;security=b.security;signing=b.signing;}
    public static Builder builder(){return new Builder();}
    public static EncodeOptions defaults(){return builder().build();}
    public static final class Builder {
        private EccLevel ecc=EccLevel.M; private CellEncoding encoding=CellEncoding.RGBW; private CompressionMode compression=CompressionMode.NONE; private Integer compressionLevel;
        private Integer version,minVersion,maxVersion; private SecurityOptions security; private SigningOptions signing;
        public Builder ecc(EccLevel v){ecc=v;return this;} public Builder cellEncoding(CellEncoding v){encoding=v;return this;} public Builder highDensity(boolean v){encoding=v?CellEncoding.HIGH_DENSITY:CellEncoding.RGBW;return this;}
        public Builder compression(CompressionMode v){compression=v;return this;} public Builder compressionLevel(int v){compressionLevel=v;return this;}
        public Builder version(int v){version=v;return this;} public Builder minVersion(int v){minVersion=v;return this;} public Builder maxVersion(int v){maxVersion=v;return this;}
        public Builder security(SecurityOptions v){security=v;return this;} public Builder signing(SigningOptions v){signing=v;return this;}
        public EncodeOptions build(){return new EncodeOptions(this);}
    }
    JSONObject toJson(){try{JSONObject o=new JSONObject().put("ecc",ecc.name()).put("cellEncoding",encoding.value).put("highDensity",encoding==CellEncoding.HIGH_DENSITY).put("compression",compression.value);if(compressionLevel!=null)o.put("compressionLevel",compressionLevel);if(version!=null)o.put("version",version);if(minVersion!=null)o.put("minVersion",minVersion);if(maxVersion!=null)o.put("maxVersion",maxVersion);if(security!=null)o.put("security",security.toJson());if(signing!=null)o.put("signing",signing.toJson());return o;}catch(Exception e){throw new QuadQRException(e.getMessage(),e);}}
}
