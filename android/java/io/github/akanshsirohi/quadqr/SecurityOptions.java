package io.github.akanshsirohi.quadqr;

import org.json.JSONObject;

public final class SecurityOptions {
    enum Mode { PASSWORD, RAW_KEY }
    final Mode mode; final String password; final byte[] key; final int iterations; final byte[] keyId; final boolean disableAutoKeyId;
    private SecurityOptions(Mode mode,String password,byte[] key,int iterations,byte[] keyId,boolean disableAutoKeyId){
        this.mode=mode;this.password=password;this.key=key;this.iterations=iterations;this.keyId=keyId;this.disableAutoKeyId=disableAutoKeyId;
    }
    public static SecurityOptions password(String password){ return password(password, 210000, null); }
    public static SecurityOptions password(String password,int iterations,byte[] keyId){
        if(password==null||password.isEmpty()) throw new IllegalArgumentException("password cannot be empty");
        return new SecurityOptions(Mode.PASSWORD,password,null,iterations,keyId,false);
    }
    public static SecurityOptions rawKey(byte[] key){ return rawKey(key,null,false); }
    public static SecurityOptions rawKey(byte[] key, byte[] keyId, boolean disableAutoKeyId){
        if(key==null||key.length!=32) throw new IllegalArgumentException("raw key must contain exactly 32 bytes");
        return new SecurityOptions(Mode.RAW_KEY,null,key.clone(),0,keyId,disableAutoKeyId);
    }
    JSONObject toJson(){
        try {
            JSONObject o=new JSONObject();
            if(mode==Mode.PASSWORD){o.put("mode","password");o.put("password",password);o.put("iterations",iterations);}
            else {o.put("mode","raw-key");o.put("key",JsonSupport.bytesArray(key));o.put("disableAutoKeyId",disableAutoKeyId);}
            if(keyId!=null)o.put("keyId",JsonSupport.bytesArray(keyId));
            return o;
        } catch(Exception e){throw new QuadQRException(e.getMessage(),e);}
    }
}
