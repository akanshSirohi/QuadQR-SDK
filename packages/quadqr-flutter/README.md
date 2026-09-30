# QuadQR Flutter Developer Guide

QuadQR Flutter is a high-level Dart SDK backed by the native QuadQR Rust engine. Flutter developers work only with Dart classes such as `QuadQR`, `EncodeOptions`, `ScanOptions`, `RenderOptions`, `SecurityOptions`, and result objects.

No native loading, platform calls, pointer management, or command-string construction is required in application code.

## 1. Requirements

- Flutter SDK
- Android min SDK 21 or newer
- One of the supported Android ABIs: `arm64-v8a`, `armeabi-v7a`, or `x86_64`

The current Flutter package targets Android. The public Dart API is kept independent of Flutter UI so additional native platforms can be added later without changing normal application code.

## 2. Prepare QuadQR Flutter

From the QuadQR repository root on Windows:

```bat
build-flutter.bat
```

The command:

1. builds the QuadQR native engine for all supported Android ABIs;
2. synchronizes the generated libraries into the Flutter package;
3. runs `flutter pub get` when Flutter is available;
4. runs `flutter analyze`.

For a distributable package archive:

```bat
build-flutter-release.bat
```

The result is:

```text
dist/quadqr-flutter.zip
```

On Linux/macOS the equivalent scripts are:

```bash
./build-flutter.sh
./build-flutter-release.sh
```

## 3. Add QuadQR to an app

During local monorepo development:

```yaml
dependencies:
  quadqr_flutter:
    path: ../QuadQR-SDK/packages/quadqr-flutter
```

Import the SDK:

```dart
import 'package:quadqr_flutter/quadqr.dart';
```

Use the shared client:

```dart
final quadQR = QuadQR.instance;
```

## 4. Basic encoding

The simplest call uses safe defaults:

```dart
final code = quadQR.encodeText('Hello Flutter');
```

`EncodedQr` contains the encoded matrix and metadata such as the selected version and CRC.

Configure the encoder with normal Dart objects:

```dart
final code = quadQR.encodeText(
  'Hello Flutter',
  options: const EncodeOptions(
    ecc: EccLevel.q,
    compression: CompressionMode.auto,
  ),
);
```

High-density mode:

```dart
final code = quadQR.encodeText(
  longText,
  options: const EncodeOptions(
    ecc: EccLevel.q,
    cellEncoding: CellEncoding.highDensity,
    compression: CompressionMode.smart,
  ),
);
```

Binary payloads:

```dart
final code = quadQR.encodeBytes(
  bytes,
  options: const EncodeOptions(
    ecc: EccLevel.m,
    compression: CompressionMode.auto,
  ),
);
```

## 5. Render PNG

```dart
final png = quadQR.renderPng(
  code,
  options: const RenderOptions(
    imageSize: 720,
    quietZone: 4,
    style: RenderStyle.classic,
  ),
);
```

The result is a `Uint8List`, so Flutter can display it directly:

```dart
Image.memory(png)
```

Available styles include:

```dart
RenderStyle.classic
RenderStyle.depth
RenderStyle.soft
RenderStyle.inset
```

## 6. Render SVG

```dart
final svg = quadQR.renderSvg(
  code,
  options: const RenderOptions(
    quietZone: 4,
    style: RenderStyle.classic,
  ),
);
```

The method returns an SVG string. Displaying SVG in Flutter is intentionally left to the application's preferred SVG widget/package.

## 7. Logo rendering

```dart
final png = quadQR.renderPng(
  code,
  logoImage: logoBytes,
  options: const RenderOptions(
    imageSize: 720,
    logo: LogoOptions(
      autoSize: true,
      clearBackground: true,
    ),
  ),
);
```

Estimate a safe logo size before rendering:

```dart
final estimate = quadQR.estimateSafeLogoSize(code);
print(estimate.safeSize);
```

Empirically test the maximum safe size against a specific logo:

```dart
final result = quadQR.findMaxSafeLogoSize(
  code,
  logoBytes,
  minSize: 0.05,
  maxSize: 0.28,
);
```

## 8. Scan an encoded image

QuadQR accepts PNG, JPEG, and WebP image bytes:

```dart
final result = quadQR.scanImage(imageBytes);

print(result.text);
print(result.scanPath);
print(result.correctedSymbols);
```

Enable recovery features explicitly when needed:

```dart
final result = quadQR.scanImage(
  imageBytes,
  options: const ScanOptions(
    finderRecovery: true,
    rotationRecovery: true,
    softDecoding: true,
  ),
);
```

## 9. Camera scanning

QuadQR deliberately does not take ownership of the Flutter camera. This keeps the SDK compatible with the camera package and application architecture you choose.

When your camera pipeline provides an RGBA frame:

```dart
final result = quadQR.scanRgba(
  rgbaBytes,
  width,
  height,
  options: const ScanOptions(
    finderRecovery: true,
    rotationRecovery: true,
    softDecoding: true,
  ),
);
```

For continuous scanning:

- keep at most one scan operation in flight;
- drop stale frames rather than creating a queue;
- avoid converting the same frame through PNG/JPEG before scanning;
- prefer direct RGBA input when your camera pipeline can provide it;
- move expensive scanning away from animation/UI work when necessary.

## 10. Matrix decoding

If you already have a sampled QuadQR matrix:

```dart
final decoded = quadQR.decodeMatrix(code.matrix);
```

Advanced soft decoding:

```dart
final decoded = quadQR.decodeMatrix(
  matrix,
  options: DecodeOptions(
    softDecoding: true,
    confidence: confidence,
    alternatives: alternatives,
  ),
);
```

Extract only payload bytes:

```dart
final bytes = quadQR.decodePayload(matrix);
```

Rotate a matrix in 90-degree steps:

```dart
final rotated = quadQR.rotateMatrix(matrix, 1);
```

## 11. Compression helpers

```dart
final compressed = quadQR.compress(
  bytes,
  CompressionAlgorithm.brotli,
  level: 8,
);

final original = quadQR.decompress(
  compressed,
  CompressionAlgorithm.brotli,
);
```

Supported standalone algorithms are LZ, DEFLATE, and Brotli.

Normal `encodeText` and `encodeBytes` calls can manage compression automatically through `CompressionMode`.

## 12. Password security

```dart
final security = SecurityOptions.password(
  'correct horse battery staple',
);

final code = quadQR.encodeText(
  'Private payload',
  options: EncodeOptions(
    compression: CompressionMode.auto,
    security: security,
  ),
);
```

After scanning:

```dart
final scan = quadQR.scanImage(imageBytes);

if (scan.requiresDecryption) {
  final payload = quadQR.decrypt(scan, security);
  print(payload.text);
}
```

## 13. Raw 256-bit keys

Generate a key:

```dart
final generated = quadQR.generateRaw256Key();
```

Use it:

```dart
final security = SecurityOptions.rawKey(generated.bytes);

final code = quadQR.encodeText(
  'Protected payload',
  options: EncodeOptions(security: security),
);
```

Persist and protect application keys according to your app's own security model. Do not hard-code production secret keys in source code.

## 14. Signing

Generate a signing key pair:

```dart
final keys = quadQR.generateSigningKeyPair();
```

Sign a payload:

```dart
final signed = quadQR.encodeText(
  'Signed payload',
  options: EncodeOptions(
    signing: SigningOptions(
      keys.privateKeySeed,
      embedPublicKey: false,
    ),
  ),
);
```

Verify a scanned payload against a trusted public key:

```dart
final scanned = quadQR.scanImage(imageBytes);

final valid = quadQR.verifySignature(
  scanned,
  trustedPublicKey: keys.publicKey,
);
```

For encrypted and signed data:

```dart
final payload = quadQR.decryptAndVerify(
  scanned,
  security,
  trustedPublicKey: keys.publicKey,
);

if (payload.verified) {
  print(payload.text);
}
```

## 15. Capacity planning

Inspect a version:

```dart
final capacity = quadQR.capacity(
  10,
  ecc: EccLevel.m,
  cellEncoding: CellEncoding.highDensity,
);
```

Plan capacity for actual bytes and options:

```dart
final plan = quadQR.calculateCapacityPlan(
  bytes,
  options: const EncodeOptions(
    ecc: EccLevel.m,
    cellEncoding: CellEncoding.highDensity,
    compression: CompressionMode.auto,
  ),
);
```

## 16. Print guidance

```dart
final guidance = quadQR.getPrintGuidance(
  code,
  options: const PrintOptions(
    physicalSizeMm: 35,
    dpi: 300,
    minimumModuleMm: 0.40,
  ),
);
```

Use the returned guidance when preparing codes for physical printing rather than assuming that a pixel size alone guarantees scanability.

## 17. Rectification and diagnostics

Rectify a detected code to a normalized PNG:

```dart
final rectified = quadQR.rectifyImage(
  imageBytes,
  moduleSize: 8,
);

final png = rectified.png;
```

For detailed scanner diagnostics:

```dart
final debug = quadQR.debugScanRgba(
  rgbaBytes,
  width,
  height,
);
```

## 18. Reliability tools

Run a reliability suite:

```dart
final report = quadQR.runReliabilityLab(
  png,
  code.crc32,
  suite: ReliabilitySuite.full,
);

print(report.score);
print(report.rating);
```

Perspective sweep:

```dart
final sweep = quadQR.runPerspectiveSweep(
  png,
  code.crc32,
  axis: PerspectiveAxis.yaw,
  angles: const [5, 10, 15, 20, 25, 30, 35, 40],
);
```

Assess an encoded QuadQR before distribution:

```dart
final assessment = quadQR.assessScanability(
  code,
  renderOptions: const RenderOptions(imageSize: 720),
  suite: ReliabilitySuite.quick,
);
```

## 19. Error handling

QuadQR operations throw `QuadQRException` for native engine failures:

```dart
try {
  final scan = quadQR.scanImage(imageBytes);
  print(scan.text);
} on QuadQRException catch (error) {
  debugPrint(error.message);
}
```

Standard Dart argument errors can still be thrown by invalid developer input, for example passing a raw security key that is not 32 bytes.

## 20. Public method map

`QuadQR.instance` exposes:

```text
version
getVersionInfo
capacity
calculateCapacityPlan

encodeText
encodeBytes
decodeMatrix
decodePayload
rotateMatrix
crc32

compress
decompress

generateRaw256Key
inspectSecurePayload
decrypt

generateSigningKeyPair
deriveSigningKeyId
signingSeedFromPkcs8
verifySignature
decryptAndVerify

renderPng
renderSvg
estimateSafeLogoSize
findMaxSafeLogoSize
getPrintGuidance

scanImage
scanRgba
debugScanRgba
rectifyImage

applyStressDistortion
runImageStressTest
runReliabilityLab
runPerspectiveSweep
assessScanability
```

## 21. Example application

A small Flutter example is included under:

```text
packages/quadqr-flutter/example
```

After running `build-flutter.bat`, you can launch it with:

```bash
cd packages/quadqr-flutter/example
flutter run
```


### Android live-camera performance

RGBA camera scans now use an adaptive fast/deep cadence. Normal frames disable finder and rotation recovery; after three consecutive misses, every fourth attempt uses the full recovery stack. The full scanner working image is capped to 960 pixels on its longest edge by default. Set `workingMaxDimension` in `ScanOptions` to override the cap, or set it to `0` to keep the original frame size. Still-image `scanImage` behavior is unchanged.

## Fast Android camera path (YUV420)

For Android `camera` plugin streams, prefer `QuadQRLiveScanner.submitYuv420(...)` over converting every frame to RGBA in Dart. The native bridge accepts the three `YUV_420_888` planes, respects row/pixel strides, converts only the bounded working image (960px longest side by default), and keeps the same adaptive fast/deep recovery cadence.

```dart
final y = image.planes[0];
final u = image.planes[1];
final v = image.planes[2];
scanner.submitYuv420(
  yPlane: y.bytes,
  uPlane: u.bytes,
  vPlane: v.bytes,
  width: image.width,
  height: image.height,
  yRowStride: y.bytesPerRow,
  yPixelStride: y.bytesPerPixel ?? 1,
  uRowStride: u.bytesPerRow,
  uPixelStride: u.bytesPerPixel ?? 1,
  vRowStride: v.bytesPerRow,
  vPixelStride: v.bytesPerPixel ?? 1,
);
```
