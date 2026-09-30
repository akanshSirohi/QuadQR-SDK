# QuadQR for Java

This guide uses the public object-oriented Android API. Application code should use `QuadQR` and the option/result classes shown below.

## 1. Create a QuadQR instance

```java
import io.github.akanshsirohi.quadqr.*;

QuadQR quadQR = QuadQR.getInstance();
```

`QuadQR` is reusable. Keep one instance in your repository, service or application component instead of creating wrappers around every call.

## 2. Encode text

The smallest call uses safe defaults:

```java
EncodedQr code = quadQR.encodeText("Hello Android");
```

For explicit settings, use `EncodeOptions`:

```java
EncodeOptions options = EncodeOptions.builder()
    .ecc(EccLevel.M)
    .highDensity(false)
    .compression(CompressionMode.AUTO)
    .build();

EncodedQr code = quadQR.encodeText("Hello Android", options);

int version = code.getVersion();
int size = code.getSize();
long crc = code.getCrc32();
int[][] matrix = code.getMatrix();
```

High-density mode:

```java
EncodedQr dense = quadQR.encodeText(
    "Higher density payload",
    EncodeOptions.builder()
        .ecc(EccLevel.Q)
        .highDensity(true)
        .compression(CompressionMode.SMART)
        .build()
);
```

You can constrain symbol versions with `.version(8)` or a range with `.minVersion(4).maxVersion(20)`.

## 3. Encode binary data

```java
byte[] payload = ...;
EncodedQr code = quadQR.encodeBytes(
    payload,
    EncodeOptions.builder()
        .ecc(EccLevel.M)
        .compression(CompressionMode.BROTLI)
        .compressionLevel(8)
        .build()
);
```

## 4. Render PNG or SVG

```java
RenderOptions render = RenderOptions.builder()
    .imageSize(720)
    .quietZone(4)
    .style(RenderStyle.CLASSIC)
    .build();

byte[] png = quadQR.renderPng(code, render);
String svg = quadQR.renderSvg(code, render);
```

Custom palette:

```java
Palette palette = Palette.builder()
    .red("#E53935")
    .green("#20A464")
    .blue("#2667FF")
    .build();

RenderOptions render = RenderOptions.builder()
    .moduleSize(10)
    .palette(palette)
    .style(RenderStyle.SOFT)
    .build();
```

### Logo rendering

```java
LogoOptions logo = LogoOptions.builder()
    .autoSize()
    .padding(0.65)
    .radius(0.8)
    .clearBackground(true)
    .build();

RenderOptions render = RenderOptions.builder()
    .imageSize(720)
    .logo(logo)
    .build();

byte[] logoBytes = ...;
byte[] png = quadQR.renderPng(code, render, logoBytes);
```

Check a recommended size before rendering:

```java
LogoSizeResult recommendation = quadQR.estimateSafeLogoSize(code, render);
double safeRatio = recommendation.getSafeSize();
```

For an empirical check using a real logo image:

```java
LogoSizeResult measured = quadQR.findMaxSafeLogoSize(
    code,
    logoBytes,
    render,
    0.05,
    0.28,
    7
);
```

## 5. Scan an image

`scanImage` accepts encoded PNG, JPEG or WebP bytes.

```java
byte[] imageBytes = ...;
ScanResult result = quadQR.scanImage(imageBytes);

String text = result.getText();
System.out.println("Decoded: " + text);
System.out.println("Path: " + result.getScanPath());
System.out.println("Corrected symbols: " + result.getCorrectedSymbols());
```

Custom scan behavior:

```java
ScanOptions scanOptions = ScanOptions.builder()
    .finderRecovery(true)
    .rotationRecovery(true)
    .softDecoding(true)
    .maxFinderCandidates(32)
    .build();

ScanResult result = quadQR.scanImage(imageBytes, scanOptions);
```

## 6. Scan camera RGBA frames

Pass RGBA bytes directly from your camera pipeline:

```java
byte[] rgba = ...;
int width = ...;
int height = ...;

ScanResult result = quadQR.scanRgba(
    rgba,
    width,
    height,
    ScanOptions.builder()
        .finderRecovery(true)
        .rotationRecovery(true)
        .build()
);
```

For continuous camera scanning, process frames on a worker thread and avoid starting multiple scans for the same frame at the same time.

## 7. Matrix decoding

```java
DecodedQr decoded = quadQR.decodeMatrix(code.getMatrix());
System.out.println(decoded.getText());
```

Binary application payload:

```java
byte[] decodedBytes = quadQR.decodePayload(
    code.getMatrix(),
    DecodeOptions.defaults()
);
```

Rotation helper:

```java
int[][] rotated = quadQR.rotateMatrix(code.getMatrix(), 1);
```

## 8. Encryption

### Password-based encryption

```java
SecurityOptions security = SecurityOptions.password("correct horse battery staple");

EncodedQr secured = quadQR.encodeText(
    "Private message",
    EncodeOptions.builder()
        .security(security)
        .compression(CompressionMode.AUTO)
        .build()
);
```

When a secure code is scanned, inspect `requiresDecryption()`:

```java
ScanResult scanned = quadQR.scanImage(imageBytes);
if (scanned.requiresDecryption()) {
    ApplicationPayload plain = quadQR.decrypt(scanned, security);
    System.out.println(plain.getText());
}
```

### Raw 256-bit key

```java
RawKey generated = quadQR.generateRaw256Key();
SecurityOptions security = SecurityOptions.rawKey(generated.getBytes());
```

Keep encryption keys in an appropriate secure storage mechanism for your application.

## 9. Ed25519 signing

Generate a signing keypair:

```java
SigningKeyPair keys = quadQR.generateSigningKeyPair();
```

Sign during encoding:

```java
SigningOptions signing = SigningOptions.builder(keys.getPrivateKeySeed())
    .embedPublicKey(false)
    .build();

EncodedQr signed = quadQR.encodeText(
    "Signed payload",
    EncodeOptions.builder()
        .signing(signing)
        .build()
);
```

Verify after scanning:

```java
ScanResult scanned = quadQR.scanImage(imageBytes);
boolean valid = quadQR.verifySignature(
    scanned,
    keys.getPublicKey(),
    false
);
```

For an encrypted and signed payload:

```java
ApplicationPayload payload = quadQR.decryptAndVerify(
    scanned,
    security,
    keys.getPublicKey(),
    false
);

if (payload.isVerified()) {
    System.out.println(payload.getText());
}
```

## 10. Compression helpers

```java
byte[] compressed = quadQR.compress(
    payload,
    CompressionAlgorithm.BROTLI,
    8
);

byte[] original = quadQR.decompress(
    compressed,
    CompressionAlgorithm.BROTLI
);
```

## 11. Capacity planning

```java
CapacityInfo v10 = quadQR.capacity(
    10,
    EccLevel.M,
    CellEncoding.RGBW
);

System.out.println(v10.getQuadQrBytes());
System.out.println(v10.getGainPercent());
```

Plan the best version for a real payload:

```java
CapacityPlan plan = quadQR.calculateCapacityPlan(
    payload,
    EncodeOptions.builder()
        .ecc(EccLevel.M)
        .highDensity(true)
        .compression(CompressionMode.AUTO)
        .build()
);

System.out.println("Version: " + plan.getQuadQrVersion());
System.out.println("Utilization: " + plan.getUtilizationPercent());
```

## 12. Print guidance

```java
PrintGuidance guidance = quadQR.getPrintGuidance(
    code,
    new PrintOptions(4, 35.0, 300.0, 0.40)
);

if (!guidance.meetsRecommendedModuleSize()) {
    System.out.println(
        "Increase physical size to at least " +
        guidance.getRecommendedMinimumPhysicalSizeMm() + " mm"
    );
}
```

## 13. Reliability tools

```java
ReliabilityReport report = quadQR.runReliabilityLab(
    png,
    code.getCrc32(),
    ReliabilitySuite.FULL,
    ScanOptions.defaults()
);

System.out.println(report.getScore());
System.out.println(report.getRating());
```

Scanability assessment:

```java
ScanabilityAssessment assessment = quadQR.assessScanability(
    code,
    render,
    ReliabilitySuite.QUICK,
    ScanOptions.defaults()
);
```

## Error handling

Native validation and decode failures are surfaced as `QuadQRException`:

```java
try {
    ScanResult result = quadQR.scanImage(imageBytes);
} catch (QuadQRException e) {
    Log.e("QuadQR", "Scan failed", e);
}
```

Use normal Android threading rules for expensive encoding, rendering, reliability tests and camera-frame processing.

## Complete Java method map

The public `QuadQR` wrapper currently exposes the full Android feature surface:

| Area | Methods |
|---|---|
| Runtime | `version`, `getVersionInfo` |
| Capacity | `capacity`, `calculateCapacityPlan` |
| Encode | `encodeText`, `encodeBytes` |
| Decode | `decodeMatrix`, `decodePayload`, `rotateMatrix`, `crc32` |
| Compression | `compress`, `decompress` |
| Encryption | `generateRaw256Key`, `inspectSecurePayload`, `decrypt` |
| Signing | `generateSigningKeyPair`, `deriveSigningKeyId`, `signingSeedFromPkcs8`, `verifySignature`, `decryptAndVerify` |
| Rendering | `renderPng`, `renderSvg`, `estimateSafeLogoSize`, `findMaxSafeLogoSize`, `getPrintGuidance` |
| Scanning | `scanImage`, `scanRgba`, `debugScanRgba`, `rectifyImage` |
| Reliability | `applyStressDistortion`, `runImageStressTest`, `runReliabilityLab`, `runPerspectiveSweep`, `assessScanability` |

Advanced matrix decoding can also supply confidence and alternative-cell matrices:

```java
DecodeOptions decodeOptions = DecodeOptions.builder()
    .softDecoding(true)
    .confidence(confidenceMatrix)
    .alternatives(alternativeMatrix)
    .build();

DecodedQr decoded = quadQR.decodeMatrix(matrix, decodeOptions);
```

If a decompressor knows the expected output size, it can provide it explicitly:

```java
byte[] original = quadQR.decompress(
    compressed,
    CompressionAlgorithm.LZ,
    expectedLength
);
```


### Android live-camera performance

RGBA camera scans now use an adaptive fast/deep cadence. Normal frames disable finder and rotation recovery; after three consecutive misses, every fourth attempt uses the full recovery stack. The full scanner working image is capped to 960 pixels on its longest edge by default. Set `workingMaxDimension` in `ScanOptions` to override the cap, or set it to `0` to keep the original frame size. Still-image `scanImage` behavior is unchanged.

## Direct YUV_420_888 camera scanning

`QuadQR.scanYuv420(...)` accepts the three Android camera planes plus their row/pixel strides. Prefer it for live CameraX/Camera2 frames when possible so the application does not first allocate a full RGBA copy. The same adaptive fast/deep camera recovery used by `scanRgba()` is applied.
