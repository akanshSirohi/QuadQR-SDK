const expected = [
  "FORMAT_VERSION", "LEGACY_FORMAT_VERSION", "MIN_VERSION", "MAX_VERSION", "DEFAULT_ECC_LEVEL",
  "RENDER_MODES", "CELL_ENCODINGS", "PRINT_PALETTE", "COMPRESSION_MODES", "LZ_LEVEL_MIN",
  "LZ_LEVEL_MAX", "DEFAULT_LZ_LEVEL", "COMPRESSION_LEVELS", "SIGNATURE_ALGORITHMS", "CELL",
  "DEFAULT_PALETTE", "RENDER_STYLES", "ECC_LEVELS", "SECURITY_MODES", "SECURITY_ALGORITHMS",
  "SECURE_PAYLOAD_VERSION", "DEFAULT_PBKDF2_ITERATIONS", "STRESS_PROFILES", "RELIABILITY_PROFILES",
  "normalizeRaw256Key", "bytesToHex", "generateRaw256Key", "generateSigningKeyPair", "deriveSigningKeyId",
  "compressPayload", "decompressPayload", "compressDeflatePayload", "decompressDeflatePayload",
  "compressBrotliPayload", "decompressBrotliPayload", "installCrc32Accelerator", "crc32",
  "getVersionInfo", "encodeText", "encodeBytes", "encodeUint8Array", "encodeSignedBytes",
  "encodeSignedText", "encodeSecureText", "encodeSecureBytes", "decryptDecoded", "decodeMatrix",
  "decodeUint8Array", "verifyDecodedSignature", "debugScanImageData", "scanImageData", "scanVideoFrame",
  "scanFile", "startCameraScanner", "rectifyDetectedCode", "rotateMatrix", "estimateSafeLogoSize",
  "findMaxSafeLogoSize", "getPrintGuidance", "renderToCanvas", "renderToImageData", "renderToSVG",
  "applyStressDistortion", "runImageStressTest", "runReliabilityLab", "runPerspectiveSweep", "assessScanability", "internals"
];

const api = await import("../library/quadqr.js");
const missing = expected.filter(name => !(name in api));
if (missing.length) {
  console.error(`Missing compatibility exports: ${missing.join(", ")}`);
  process.exit(1);
}

const code = api.encodeText("QuadQR Rust/WASM compatibility check", { ecc: "M" });
if (!code || Object.getPrototypeOf(code) !== Object.prototype) {
  throw new Error("encodeText() must return a plain JavaScript object, not Map or another wrapper type.");
}
if (!Array.isArray(code.matrix) || !Array.isArray(code.matrix[0])) {
  throw new Error("encodeText().matrix must be a normal 2D JavaScript array.");
}
const decoded = api.decodeMatrix(code.matrix);
if (decoded.text !== "QuadQR Rust/WASM compatibility check") {
  throw new Error("RGBW encode/decode compatibility smoke test failed.");
}

const dense = api.encodeText("Triangle16 WASM", { ecc: "M", highDensity: true });
const denseDecoded = api.decodeMatrix(dense.matrix);
if (denseDecoded.text !== "Triangle16 WASM") {
  throw new Error("Triangle16 encode/decode compatibility smoke test failed.");
}

const compressed = api.compressPayload(new TextEncoder().encode("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
const restored = api.decompressPayload(compressed, 32);
if (new TextDecoder().decode(restored) !== "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa") {
  throw new Error("LZ compression compatibility smoke test failed.");
}

const benchmark = await import("../library/benchmark.js");
if (typeof benchmark.buildCapacityComparison !== "function" || typeof benchmark.calculateCapacityPlan !== "function") {
  throw new Error("Benchmark compatibility exports are incomplete.");
}

console.log(`QuadQR JS compatibility surface OK: ${expected.length} expected exports present.`);
console.log(`WASM backend: ${(await import("../library/wasm.js")).getWasmState().backend}`);
