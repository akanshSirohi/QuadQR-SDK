package io.github.akanshsirohi.quadqr;
public enum CompressionMode {
    NONE("none"), AUTO("auto"), SMART("smart"), LZ("lz"), DEFLATE("deflate"), BROTLI("brotli");
    final String value; CompressionMode(String value){this.value=value;}
}
