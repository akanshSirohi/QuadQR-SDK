# QuadQR for Kotlin

QuadQR's Android wrapper is directly usable from Kotlin with typed enums, builders and result objects. Optional Kotlin extension helpers are included for compact configuration blocks.

## Setup

```kotlin
import io.github.akanshsirohi.quadqr.*

val quadQR = QuadQR.getInstance()
```

## Encode text

```kotlin
val code = quadQR.encodeText("Hello Android")
```

With options:

```kotlin
val code = quadQR.encodeText(
    "Hello Android",
    EncodeOptions.builder()
        .ecc(EccLevel.M)
        .highDensity(false)
        .compression(CompressionMode.AUTO)
        .build()
)

println(code.version)
println(code.crc32)
```

If you include `QuadQRKtx.kt`, the configuration-block overload is available:

```kotlin
val code = quadQR.encodeText("Hello Android") {
    ecc(EccLevel.M)
    highDensity(true)
    compression(CompressionMode.SMART)
}
```

## Render PNG

```kotlin
val png: ByteArray = quadQR.renderPng(
    code,
    RenderOptions.builder()
        .imageSize(720)
        .quietZone(4)
        .style(RenderStyle.CLASSIC)
        .build()
)
```

With a logo:

```kotlin
val render = RenderOptions.builder()
    .imageSize(720)
    .logo(
        LogoOptions.builder()
            .autoSize()
            .clearBackground(true)
            .build()
    )
    .build()

val png = quadQR.renderPng(code, render, logoBytes)
```

## Scan an image

```kotlin
val scan = quadQR.scanImage(imageBytes)
println(scan.text)
println(scan.correctedSymbols)
println(scan.scanPath)
```

Custom scanner:

```kotlin
val scan = quadQR.scanImage(
    imageBytes,
    ScanOptions.builder()
        .finderRecovery(true)
        .rotationRecovery(true)
        .softDecoding(true)
        .build()
)
```

Or with the Kotlin extension:

```kotlin
val scan = quadQR.scanImage(imageBytes) {
    finderRecovery(true)
    rotationRecovery(true)
    softDecoding(true)
}
```

## Camera frames

```kotlin
val result = quadQR.scanRgba(
    rgba,
    width,
    height,
    ScanOptions.builder()
        .finderRecovery(true)
        .rotationRecovery(true)
        .build()
)
```

For CameraX, feed analysis frames from a background executor. Keep only one active scan per frame source unless your application deliberately manages parallel work.

## Encryption

```kotlin
val security = SecurityOptions.password("my-password")

val secureCode = quadQR.encodeText(
    "Private payload",
    EncodeOptions.builder()
        .security(security)
        .compression(CompressionMode.AUTO)
        .build()
)
```

Decrypt after scanning:

```kotlin
val scan = quadQR.scanImage(imageBytes)
if (scan.requiresDecryption()) {
    val payload = quadQR.decrypt(scan, security)
    println(payload.text)
}
```

Raw key:

```kotlin
val key = quadQR.generateRaw256Key()
val security = SecurityOptions.rawKey(key.bytes)
```

## Signing

```kotlin
val keys = quadQR.generateSigningKeyPair()

val signing = SigningOptions.builder(keys.privateKeySeed)
    .embedPublicKey(false)
    .build()

val signedCode = quadQR.encodeText(
    "Signed message",
    EncodeOptions.builder()
        .signing(signing)
        .build()
)
```

Verify:

```kotlin
val scan = quadQR.scanImage(imageBytes)
val valid = quadQR.verifySignature(
    scan,
    keys.publicKey,
    false
)
```

Encrypted + signed:

```kotlin
val payload = quadQR.decryptAndVerify(
    scan,
    security,
    keys.publicKey,
    false
)

if (payload.isVerified) {
    println(payload.text)
}
```

## Matrix operations

```kotlin
val decoded = quadQR.decodeMatrix(code.matrix)
val rotated = quadQR.rotateMatrix(code.matrix, 1)
val checksum = quadQR.crc32(decoded.payload)
```

## Capacity planning

```kotlin
val capacity = quadQR.capacity(
    10,
    EccLevel.M,
    CellEncoding.HIGH_DENSITY
)

val plan = quadQR.calculateCapacityPlan(
    data,
    EncodeOptions.builder()
        .ecc(EccLevel.M)
        .highDensity(true)
        .compression(CompressionMode.AUTO)
        .build()
)
```

## Print guidance

```kotlin
val guidance = quadQR.getPrintGuidance(
    code,
    PrintOptions(
        4,
        35.0,
        300.0,
        0.40
    )
)
```

## Reliability

```kotlin
val report = quadQR.runReliabilityLab(
    png,
    code.crc32,
    ReliabilitySuite.FULL,
    ScanOptions.defaults()
)

println(report.score)
println(report.rating)
```

## Error handling

```kotlin
try {
    val result = quadQR.scanImage(imageBytes)
} catch (e: QuadQRException) {
    Log.e("QuadQR", "Scan failed", e)
}
```

Use coroutines or your normal background dispatcher for continuous camera scanning and heavier reliability operations.

## Complete Kotlin method map

The same `QuadQR` instance provides runtime/version information, capacity planning, text and binary encoding, matrix decoding, compression, encryption, signing, PNG/SVG rendering, logo safety tools, image scanning, RGBA camera scanning, rectification and reliability analysis.

The complete callable surface is:

```text
version / getVersionInfo
capacity / calculateCapacityPlan
encodeText / encodeBytes
decodeMatrix / decodePayload / rotateMatrix / crc32
compress / decompress
generateRaw256Key / inspectSecurePayload / decrypt
generateSigningKeyPair / deriveSigningKeyId / signingSeedFromPkcs8
verifySignature / decryptAndVerify
renderPng / renderSvg / estimateSafeLogoSize / findMaxSafeLogoSize / getPrintGuidance
scanImage / scanRgba / debugScanRgba / rectifyImage
applyStressDistortion / runImageStressTest / runReliabilityLab
runPerspectiveSweep / assessScanability
```

For advanced decoding, `DecodeOptions.Builder` also accepts confidence and alternative-cell matrices. This is useful when your own sampling pipeline has per-cell confidence information.


### Android live-camera performance

RGBA camera scans now use an adaptive fast/deep cadence. Normal frames disable finder and rotation recovery; after three consecutive misses, every fourth attempt uses the full recovery stack. The full scanner working image is capped to 960 pixels on its longest edge by default. Set `workingMaxDimension` in `ScanOptions` to override the cap, or set it to `0` to keep the original frame size. Still-image `scanImage` behavior is unchanged.

## Direct YUV_420_888 camera scanning

For live CameraX/Camera2 frames, prefer `QuadQR.get().scanYuv420(...)` with the Y/U/V plane bytes and their row/pixel strides. This avoids a full application-side RGBA conversion and keeps the native live-camera working image bounded.
