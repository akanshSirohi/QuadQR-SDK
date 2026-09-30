package io.github.akanshsirohi.quadqr;

import org.json.JSONObject;

public class DecodeOptions {
    final Double structuralTolerance,maxErasureConfidence,softConfidenceThreshold; final Boolean softDecoding; final Integer softMaxCells,softPairCells; final float[][] confidence; final int[][] alternatives;
    protected DecodeOptions(Builder<?> b){structuralTolerance=b.structuralTolerance;maxErasureConfidence=b.maxErasureConfidence;softConfidenceThreshold=b.softConfidenceThreshold;softDecoding=b.softDecoding;softMaxCells=b.softMaxCells;softPairCells=b.softPairCells;confidence=b.confidence;alternatives=b.alternatives;}
    public static Builder<?> builder(){return new Builder<>();}
    public static DecodeOptions defaults(){return builder().build();}
    public static class Builder<T extends Builder<T>> {
        Double structuralTolerance,maxErasureConfidence,softConfidenceThreshold; Boolean softDecoding; Integer softMaxCells,softPairCells; float[][] confidence; int[][] alternatives;
        @SuppressWarnings("unchecked") T self(){return (T)this;}
        public T structuralTolerance(double v){structuralTolerance=v;return self();} public T maxErasureConfidence(double v){maxErasureConfidence=v;return self();}
        public T softDecoding(boolean v){softDecoding=v;return self();} public T softConfidenceThreshold(double v){softConfidenceThreshold=v;return self();}
        public T softMaxCells(int v){softMaxCells=v;return self();} public T softPairCells(int v){softPairCells=v;return self();} public T confidence(float[][] v){confidence=v;return self();} public T alternatives(int[][] v){alternatives=v;return self();}
        public DecodeOptions build(){return new DecodeOptions(this);}
    }
    JSONObject toJson(){try{JSONObject o=new JSONObject();if(structuralTolerance!=null)o.put("structuralTolerance",structuralTolerance);if(maxErasureConfidence!=null)o.put("maxErasureConfidence",maxErasureConfidence);if(softDecoding!=null)o.put("softDecoding",softDecoding);if(softConfidenceThreshold!=null)o.put("softConfidenceThreshold",softConfidenceThreshold);if(softMaxCells!=null)o.put("softMaxCells",softMaxCells);if(softPairCells!=null)o.put("softPairCells",softPairCells);if(confidence!=null){org.json.JSONArray rows=new org.json.JSONArray();for(float[] row:confidence){org.json.JSONArray r=new org.json.JSONArray();for(float v:row)r.put(v);rows.put(r);}o.put("confidence",rows);}if(alternatives!=null)o.put("alternatives",JsonSupport.matrix(alternatives));return o;}catch(Exception e){throw new QuadQRException(e.getMessage(),e);}}
}
