package io.github.akanshsirohi.quadqr;

import android.util.Base64;
import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

final class JsonSupport {
    private JsonSupport() {}

    static JSONObject result(String response) {
        try {
            JSONObject envelope = new JSONObject(response);
            if (!envelope.optBoolean("ok", false)) {
                throw new QuadQRException(envelope.optString("error", "QuadQR operation failed"));
            }
            Object result = envelope.opt("result");
            return result instanceof JSONObject ? (JSONObject) result : new JSONObject().put("value", result);
        } catch (JSONException e) {
            throw new QuadQRException("Invalid response from QuadQR native engine", e);
        }
    }

    static byte[] bytes(JSONObject object) {
        String base64 = object.optString("dataBase64", "");
        return Base64.decode(base64, Base64.DEFAULT);
    }

    static JSONArray bytesArray(byte[] bytes) {
        JSONArray a = new JSONArray();
        if (bytes != null) for (byte b : bytes) a.put(b & 0xff);
        return a;
    }

    static byte[] byteArray(JSONArray a) {
        if (a == null) return new byte[0];
        byte[] out = new byte[a.length()];
        for (int i=0;i<a.length();i++) out[i]=(byte)a.optInt(i,0);
        return out;
    }

    static JSONArray matrix(int[][] matrix) {
        JSONArray rows = new JSONArray();
        if (matrix != null) for (int[] row : matrix) {
            JSONArray r = new JSONArray();
            for (int v : row) r.put(v);
            rows.put(r);
        }
        return rows;
    }

    static int[][] matrix(JSONArray rows) {
        if (rows == null) return new int[0][0];
        int[][] out = new int[rows.length()][];
        for (int i=0;i<rows.length();i++) {
            JSONArray row = rows.optJSONArray(i);
            if (row == null) { out[i] = new int[0]; continue; }
            out[i] = new int[row.length()];
            for (int x=0;x<row.length();x++) out[i][x]=row.optInt(x);
        }
        return out;
    }

    static JSONObject merge(JSONObject target, JSONObject source) {
        if (source == null) return target;
        JSONArray names = source.names();
        if (names == null) return target;
        for (int i=0;i<names.length();i++) {
            String key = names.optString(i);
            try { target.put(key, source.opt(key)); } catch (JSONException e) { throw new QuadQRException(e.getMessage(), e); }
        }
        return target;
    }
}
