package io.github.akanshsirohi.quadqr;

import org.json.JSONObject;

public final class SigningOptions {
    final byte[] privateKeySeed; final byte[] keyId; final boolean embedPublicKey;
    private SigningOptions(Builder b){privateKeySeed=b.privateKeySeed.clone();keyId=b.keyId==null?null:b.keyId.clone();embedPublicKey=b.embedPublicKey;}
    public static Builder builder(byte[] privateKeySeed){return new Builder(privateKeySeed);}
    public static final class Builder {
        private final byte[] privateKeySeed; private byte[] keyId; private boolean embedPublicKey;
        private Builder(byte[] seed){if(seed==null||seed.length!=32)throw new IllegalArgumentException("Ed25519 seed must contain 32 bytes");privateKeySeed=seed.clone();}
        public Builder keyId(byte[] value){keyId=value==null?null:value.clone();return this;}
        public Builder embedPublicKey(boolean value){embedPublicKey=value;return this;}
        public SigningOptions build(){return new SigningOptions(this);}
    }
    JSONObject toJson(){try{JSONObject o=new JSONObject().put("privateKeySeed",JsonSupport.bytesArray(privateKeySeed)).put("embedPublicKey",embedPublicKey);if(keyId!=null)o.put("keyId",JsonSupport.bytesArray(keyId));return o;}catch(Exception e){throw new QuadQRException(e.getMessage(),e);}}
}
