package io.github.akanshsirohi.quadqr;

/** Thrown when the QuadQR engine rejects input or cannot complete an operation. */
public final class QuadQRException extends RuntimeException {
    public QuadQRException(String message) { super(message); }
    public QuadQRException(String message, Throwable cause) { super(message, cause); }
}
