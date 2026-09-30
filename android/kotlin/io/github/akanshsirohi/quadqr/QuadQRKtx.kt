package io.github.akanshsirohi.quadqr

/** Kotlin convenience overloads on top of the same object-oriented Android API. */
inline fun QuadQR.encodeText(text: String, configure: EncodeOptions.Builder.() -> Unit): EncodedQr {
    val builder = EncodeOptions.builder()
    builder.configure()
    return encodeText(text, builder.build())
}

inline fun QuadQR.scanRgba(rgba: ByteArray, width: Int, height: Int, configure: ScanOptions.Builder.() -> Unit): ScanResult {
    val builder = ScanOptions.builder()
    builder.configure()
    return scanRgba(rgba, width, height, builder.build())
}

inline fun QuadQR.scanImage(image: ByteArray, configure: ScanOptions.Builder.() -> Unit): ScanResult {
    val builder = ScanOptions.builder()
    builder.configure()
    return scanImage(image, builder.build())
}

inline fun QuadQR.renderPng(code: EncodedQr, configure: RenderOptions.Builder.() -> Unit): ByteArray {
    val builder = RenderOptions.builder()
    builder.configure()
    return renderPng(code, builder.build())
}
