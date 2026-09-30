import { getWasm, getWasmState, initWasm } from "./wasm.js";

export { initWasm } from "./wasm.js";

export const FORMAT_VERSION = 6;
export const LEGACY_FORMAT_VERSION = 5;
export const MIN_VERSION = 1;
export const MAX_VERSION = 40;
export const DEFAULT_ECC_LEVEL = "M";

export const RENDER_MODES = Object.freeze({ SCREEN: "screen", PRINT: "print" });
export const CELL_ENCODINGS = Object.freeze({ RGBW: "rgbw", TRIANGLE16: "triangle16" });
export const COMPRESSION_MODES = Object.freeze({
  NONE: "none", AUTO: "auto", SMART: "smart", LZ: "lz", DEFLATE: "deflate", BROTLI: "brotli"
});
export const LZ_LEVEL_MIN = 1;
export const LZ_LEVEL_MAX = 9;
export const DEFAULT_LZ_LEVEL = 6;
export const COMPRESSION_LEVELS = Object.freeze({
  lz: Object.freeze({ min: 1, max: 9, default: 6 }),
  deflate: Object.freeze({ min: 1, max: 9, default: 6 }),
  brotli: Object.freeze({ min: 0, max: 11, default: 11 }),
  auto: Object.freeze({ lz: 6, deflate: 6, brotli: 6 }),
  smart: Object.freeze({
    initialLz: 6,
    initialDeflate: 6,
    initialBrotli: 6,
    strongDeflate: 8,
    strongBrotli: 9,
    maximumDeflate: 9,
    maximumBrotli: 11
  })
});
export const SIGNATURE_ALGORITHMS = Object.freeze({ ED25519: "Ed25519" });
export const CELL = Object.freeze({ BLACK: -1, RED: 0, GREEN: 1, BLUE: 2, WHITE: 3 });
export const DEFAULT_PALETTE = Object.freeze({
  black: "#000000", white: "#ffffff", red: "#ef233c", green: "#16a34a", blue: "#2563eb"
});
export const PRINT_PALETTE = Object.freeze({
  black: "#000000", white: "#ffffff", red: "#d71932", green: "#087f3e", blue: "#174ea6"
});
export const RENDER_STYLES = Object.freeze({ CLASSIC: "classic", DEPTH: "depth", SOFT: "soft", INSET: "inset" });
export const ECC_LEVELS = Object.freeze({
  L: Object.freeze({ id: 0, paritySymbols: 12, correctableSymbolsPerBlock: 6 }),
  M: Object.freeze({ id: 1, paritySymbols: 24, correctableSymbolsPerBlock: 12 }),
  Q: Object.freeze({ id: 2, paritySymbols: 36, correctableSymbolsPerBlock: 18 }),
  H: Object.freeze({ id: 3, paritySymbols: 48, correctableSymbolsPerBlock: 24 })
});

export const SECURITY_MODES = Object.freeze({ PASSWORD: "password", RAW_KEY: "raw-key" });
export const SECURITY_ALGORITHMS = Object.freeze({ AES_256_GCM: "AES-256-GCM" });
export const SECURE_PAYLOAD_VERSION = 1;
export const DEFAULT_PBKDF2_ITERATIONS = 600000;

export const STRESS_PROFILES = Object.freeze([
  Object.freeze({ id: "clean", label: "Clean", type: "clean", severity: 0, weight: 2 }),
  Object.freeze({ id: "blur", label: "Blur", type: "blur", severity: 0.42, weight: 1 }),
  Object.freeze({ id: "dark", label: "Low brightness", type: "brightness-low", severity: 0.48, weight: 1 }),
  Object.freeze({ id: "bright", label: "High exposure", type: "brightness-high", severity: 0.34, weight: 1 }),
  Object.freeze({ id: "shadow", label: "Uneven shadow", type: "shadow", severity: 0.55, weight: 1.2 }),
  Object.freeze({ id: "contrast", label: "Contrast loss", type: "contrast-loss", severity: 0.42, weight: 1 }),
  Object.freeze({ id: "perspective", label: "Perspective", type: "perspective", severity: 0.42, weight: 1.3 }),
  Object.freeze({ id: "jpeg", label: "JPEG-like compression", type: "jpeg", severity: 0.50, weight: 0.8 }),
  Object.freeze({ id: "downscale", label: "Downscale", type: "downscale", severity: 0.52, weight: 0.9 })
]);

export const RELIABILITY_PROFILES = Object.freeze([
  Object.freeze({ id: "clean", label: "Clean reference", category: "Baseline", type: "clean", severity: 0, weight: 2, suite: "quick" }),
  Object.freeze({ id: "blur", label: "Lens blur", category: "Optics", type: "blur", severity: 0.40, weight: 1, suite: "quick" }),
  Object.freeze({ id: "motion", label: "Motion blur", category: "Optics", type: "motion-blur", severity: 0.42, weight: 1, suite: "full" }),
  Object.freeze({ id: "dark", label: "Low light", category: "Exposure", type: "brightness-low", severity: 0.50, weight: 1, suite: "quick" }),
  Object.freeze({ id: "bright", label: "High exposure", category: "Exposure", type: "brightness-high", severity: 0.38, weight: 1, suite: "full" }),
  Object.freeze({ id: "shadow", label: "Gradient shadow", category: "Lighting", type: "shadow", severity: 0.58, weight: 1.2, suite: "quick" }),
  Object.freeze({ id: "glare", label: "Specular glare", category: "Lighting", type: "glare", severity: 0.34, weight: 1.1, suite: "full" }),
  Object.freeze({ id: "warm", label: "Warm color cast", category: "Color", type: "warm", severity: 0.62, weight: 1, suite: "quick" }),
  Object.freeze({ id: "cool", label: "Cool color cast", category: "Color", type: "cool", severity: 0.60, weight: 1, suite: "full" }),
  Object.freeze({ id: "contrast", label: "Contrast loss", category: "Color", type: "contrast-loss", severity: 0.46, weight: 1, suite: "full" }),
  Object.freeze({ id: "noise", label: "Sensor noise", category: "Sensor", type: "noise", severity: 0.42, weight: 0.9, suite: "full" }),
  Object.freeze({ id: "jpeg", label: "JPEG-like damage", category: "Resampling", type: "jpeg", severity: 0.52, weight: 0.8, suite: "full" }),
  Object.freeze({ id: "downscale", label: "Aggressive downscale", category: "Resampling", type: "downscale", severity: 0.55, weight: 1, suite: "quick" }),
  Object.freeze({ id: "perspective-2d", label: "Projective skew", category: "Perspective", type: "perspective", severity: 0.52, weight: 1.3, suite: "quick" }),
  Object.freeze({ id: "yaw-35", label: "3D yaw 35°", category: "Perspective", type: "perspective-3d", severity: 0.55, yawDegrees: 35, pitchDegrees: 0, rollDegrees: 0, weight: 1.4, suite: "quick" }),
  Object.freeze({ id: "pitch-30", label: "3D pitch 30°", category: "Perspective", type: "perspective-3d", severity: 0.50, yawDegrees: 0, pitchDegrees: 30, rollDegrees: 0, weight: 1.3, suite: "full" }),
  Object.freeze({ id: "z-rotation-55", label: "Z rotation 55°", category: "Perspective", type: "perspective-3d", severity: 0.45, yawDegrees: 0, pitchDegrees: 0, rollDegrees: 55, weight: 1.1, suite: "quick" }),
  Object.freeze({ id: "combined-3d", label: "Combined 3D tilt", category: "Perspective", type: "perspective-3d", severity: 0.62, yawDegrees: 30, pitchDegrees: 15, rollDegrees: 18, weight: 1.5, suite: "full" }),
  Object.freeze({ id: "yaw-55", label: "Extreme yaw 55°", category: "Extreme perspective", type: "perspective-3d", severity: 0.82, yawDegrees: 55, pitchDegrees: 0, rollDegrees: 0, weight: 1.5, suite: "extreme" }),
  Object.freeze({ id: "pitch-40", label: "Extreme pitch 40°", category: "Extreme perspective", type: "perspective-3d", severity: 0.78, yawDegrees: 0, pitchDegrees: 40, rollDegrees: 0, weight: 1.4, suite: "extreme" }),
  Object.freeze({ id: "z-rotation-75", label: "Z rotation 75°", category: "Extreme perspective", type: "perspective-3d", severity: 0.72, yawDegrees: 0, pitchDegrees: 0, rollDegrees: 75, weight: 1.1, suite: "extreme" })
]);

function wasm() {
  return getWasm();
}

function asBytes(input) {
  if (input instanceof Uint8Array) return input;
  if (input instanceof Uint8ClampedArray) return new Uint8Array(input.buffer, input.byteOffset, input.byteLength);
  if (input instanceof ArrayBuffer) return new Uint8Array(input);
  if (ArrayBuffer.isView(input)) return new Uint8Array(input.buffer, input.byteOffset, input.byteLength);
  if (Array.isArray(input)) return Uint8Array.from(input);
  throw new TypeError("Expected Uint8Array-compatible input.");
}

function normalizeByteFields(value) {
  if (!value || typeof value !== "object") return value;
  for (const key of ["payload", "protectedPayload", "publicKeyBytes", "privateKeyPkcs8", "privateKeySeed"]) {
    if (Array.isArray(value[key])) value[key] = Uint8Array.from(value[key]);
  }
  if (value.imageData?.data && Array.isArray(value.imageData.data)) {
    value.imageData.data = Uint8ClampedArray.from(value.imageData.data);
  }
  return value;
}

function normalizeCode(value, options = {}) {
  const code = normalizeByteFields(value);
  if (!code || typeof code !== "object") return code;
  const requested = String(options.compression ?? "none").toLowerCase();
  if (requested === "auto" || requested === "smart") {
    code.compressionStrategy = requested;
  }
  return code;
}

function normalizeDecoded(value) {
  return normalizeByteFields(value);
}

function copyOptions(options = {}) {
  return options && typeof options === "object" ? { ...options } : {};
}

// serde-wasm-bindgen handles normal JS values directly, but converting typed
// byte arrays inside nested option objects to plain arrays keeps the bridge
// deterministic across browsers and Node versions.
function wasmOptions(value) {
  if (value == null) return value;
  if (value instanceof Uint8Array || value instanceof Uint8ClampedArray || ArrayBuffer.isView(value)) {
    return Array.from(new Uint8Array(value.buffer, value.byteOffset, value.byteLength));
  }
  if (value instanceof ArrayBuffer) return Array.from(new Uint8Array(value));
  if (Array.isArray(value)) return value.map(wasmOptions);
  if (typeof value === "object") {
    const out = {};
    for (const [key, item] of Object.entries(value)) out[key] = wasmOptions(item);
    return out;
  }
  return value;
}

export function getVersionInfo(version, options = {}) {
  return wasm().getVersionInfo(Number(version), wasmOptions(copyOptions(options)));
}

export function encodeText(text, options = {}) {
  return normalizeCode(wasm().encodeText(String(text), wasmOptions(copyOptions(options))), options);
}

export function encodeBytes(input, options = {}) {
  return normalizeCode(wasm().encodeBytes(asBytes(input), wasmOptions(copyOptions(options))), options);
}

export function encodeUint8Array(input, options = {}) {
  return encodeBytes(input, options);
}

export function decodeMatrix(inputMatrix, options = {}) {
  return normalizeDecoded(wasm().decodeMatrix(inputMatrix, wasmOptions(copyOptions(options))));
}

export function decodeUint8Array(inputMatrix, options = {}) {
  return decodeMatrix(inputMatrix, options).payload;
}

export function rotateMatrix(matrix, quarterTurns = 1) {
  return wasm().rotateMatrix(matrix, Number(quarterTurns) || 0);
}

let crc32Accelerator = null;
export function installCrc32Accelerator(accelerator = null) {
  crc32Accelerator = typeof accelerator === "function" ? accelerator : null;
  return Boolean(crc32Accelerator);
}

export function crc32(bytes) {
  const input = asBytes(bytes);
  if (crc32Accelerator) return crc32Accelerator(input) >>> 0;
  return wasm().crc32(input) >>> 0;
}

export function compressPayload(input, options = {}) {
  return Uint8Array.from(wasm().compressLz(asBytes(input), Number(options.level ?? DEFAULT_LZ_LEVEL)));
}

export function decompressPayload(input, expectedLength = null) {
  return Uint8Array.from(wasm().decompressLz(asBytes(input), expectedLength == null ? undefined : Number(expectedLength)));
}

export function compressDeflatePayload(input, options = {}) {
  return Uint8Array.from(wasm().compressDeflate(asBytes(input), Number(options.level ?? 6)));
}

export function decompressDeflatePayload(input, expectedLength = null) {
  return Uint8Array.from(wasm().decompressDeflate(asBytes(input), expectedLength == null ? undefined : Number(expectedLength)));
}

export function compressBrotliPayload(input, options = {}) {
  return Uint8Array.from(wasm().compressBrotli(asBytes(input), Number(options.quality ?? options.level ?? 11)));
}

export function decompressBrotliPayload(input, expectedLength = null) {
  return Uint8Array.from(wasm().decompressBrotli(asBytes(input), expectedLength == null ? undefined : Number(expectedLength)));
}

export function generateRaw256Key() {
  if (globalThis.crypto?.getRandomValues) {
    return globalThis.crypto.getRandomValues(new Uint8Array(32));
  }
  return Uint8Array.from(wasm().generateRaw256Key());
}

export function bytesToHex(input) {
  return Array.from(asBytes(input), byte => byte.toString(16).padStart(2, "0")).join("");
}

export function normalizeRaw256Key(input) {
  if (typeof input === "string") {
    const clean = input.trim();
    if (!/^[0-9a-fA-F]{64}$/.test(clean)) throw new Error("Raw 256-bit key must contain exactly 64 hexadecimal characters.");
    return Uint8Array.from(clean.match(/../g).map(part => parseInt(part, 16)));
  }
  const bytes = asBytes(input);
  if (bytes.length !== 32) throw new Error("Raw 256-bit key must contain exactly 32 bytes.");
  return new Uint8Array(bytes);
}

async function exportPrivateSeed(privateKey) {
  if (privateKey instanceof Uint8Array || ArrayBuffer.isView(privateKey) || privateKey instanceof ArrayBuffer || Array.isArray(privateKey)) {
    const bytes = asBytes(privateKey);
    if (bytes.length === 32) return new Uint8Array(bytes);
    return Uint8Array.from(wasm().signingSeedFromPkcs8(bytes));
  }
  if (globalThis.CryptoKey && privateKey instanceof CryptoKey) {
    const pkcs8 = new Uint8Array(await crypto.subtle.exportKey("pkcs8", privateKey));
    return Uint8Array.from(wasm().signingSeedFromPkcs8(pkcs8));
  }
  throw new Error("Ed25519 privateKey must be a CryptoKey, 32-byte seed, or PKCS#8 byte array.");
}

async function exportPublicBytes(publicKey) {
  if (publicKey instanceof Uint8Array || ArrayBuffer.isView(publicKey) || publicKey instanceof ArrayBuffer || Array.isArray(publicKey)) {
    const bytes = asBytes(publicKey);
    if (bytes.length !== 32) throw new Error("Ed25519 public key must contain exactly 32 bytes.");
    return new Uint8Array(bytes);
  }
  if (globalThis.CryptoKey && publicKey instanceof CryptoKey) {
    return new Uint8Array(await crypto.subtle.exportKey("raw", publicKey));
  }
  throw new Error("Ed25519 publicKey must be a CryptoKey or 32 raw bytes.");
}

export async function deriveSigningKeyId(publicKey, bytes = 8) {
  const publicKeyBytes = await exportPublicBytes(publicKey);
  return bytesToHex(Uint8Array.from(wasm().deriveSigningKeyId(publicKeyBytes, Number(bytes))));
}

export async function generateSigningKeyPair() {
  if (!globalThis.crypto?.subtle) throw new Error("Web Crypto is required for Ed25519 key generation.");
  const pair = await crypto.subtle.generateKey({ name: "Ed25519" }, true, ["sign", "verify"]);
  const publicKeyBytes = new Uint8Array(await crypto.subtle.exportKey("raw", pair.publicKey));
  const privateKeyPkcs8 = new Uint8Array(await crypto.subtle.exportKey("pkcs8", pair.privateKey));
  const keyId = await deriveSigningKeyId(publicKeyBytes, 8);
  return {
    publicKey: pair.publicKey,
    privateKey: pair.privateKey,
    publicKeyBytes,
    privateKeyPkcs8,
    keyId
  };
}

async function normalizeSigningOptions(options = {}) {
  const privateKey = options.privateKey ?? options.signing?.privateKey;
  if (!privateKey) throw new Error("A privateKey is required for Ed25519 signing.");
  return {
    privateKeySeed: await exportPrivateSeed(privateKey),
    keyId: options.keyId ?? options.signing?.keyId ?? null,
    embedPublicKey: Boolean(options.embedPublicKey ?? options.signing?.embedPublicKey)
  };
}

function normalizeSecurityForWasm(security) {
  if (!security || security.mode === "none") return null;
  if (security.mode === "password") {
    return {
      mode: "password",
      password: String(security.password ?? ""),
      iterations: Number(security.iterations ?? DEFAULT_PBKDF2_ITERATIONS),
      keyId: security.keyId ?? null
    };
  }
  if (security.mode === "raw-key") {
    return {
      mode: "raw-key",
      key: bytesToHex(normalizeRaw256Key(security.key)),
      keyId: security.keyId ?? null,
      disableAutoKeyId: Boolean(security.disableAutoKeyId)
    };
  }
  throw new Error("security.mode must be none, password, or raw-key.");
}

export async function encodeSignedBytes(input, options = {}) {
  const signing = await normalizeSigningOptions(options);
  return encodeBytes(input, { ...options, signing });
}

export async function encodeSignedText(text, options = {}) {
  const signing = await normalizeSigningOptions(options);
  return encodeText(text, { ...options, signing });
}

export async function encodeSecureBytes(input, options = {}) {
  const security = normalizeSecurityForWasm(options.security);
  let signing = null;
  if (options.signing) signing = await normalizeSigningOptions({ signing: options.signing });
  return encodeBytes(input, { ...options, security, ...(signing ? { signing } : {}) });
}

export async function encodeSecureText(text, options = {}) {
  const security = normalizeSecurityForWasm(options.security);
  let signing = null;
  if (options.signing) signing = await normalizeSigningOptions({ signing: options.signing });
  return encodeText(text, { ...options, security, ...(signing ? { signing } : {}) });
}

export async function decryptDecoded(result, security = {}) {
  if (!result?.secure || !result?.requiresDecryption) return result;
  const mode = result.security?.mode === "raw-key" ? "raw-key" : "password";
  const credentials = normalizeSecurityForWasm({ mode, ...security });
  const unlocked = normalizeDecoded(wasm().decryptPayload(
    asBytes(result.payload),
    Number(result.flags ?? 0),
    Boolean(result.extended),
    credentials
  ));
  return {
    ...result,
    ...unlocked,
    secure: true,
    requiresDecryption: false,
    decrypted: true,
    security: { ...result.security, decrypted: true }
  };
}

export async function verifyDecodedSignature(result, options = {}) {
  if (!result?.signed) return result;
  const protectedPayload = result.protectedPayload;
  if (!protectedPayload) throw new Error("Signed payload bytes are unavailable for verification.");
  const publicKey = options.publicKey ? await exportPublicBytes(options.publicKey) : null;
  const valid = wasm().verifySignedPayload(
    asBytes(protectedPayload),
    publicKey ?? new Uint8Array(),
    Boolean(options.allowEmbeddedKey ?? options.allowEmbeddedPublicKey)
  );
  if (!valid) throw new Error("Ed25519 signature verification failed.");
  return {
    ...result,
    signatureVerified: true,
    signatureTrusted: Boolean(publicKey),
  };
}

function ensureImageDataLike(imageData) {
  if (!imageData?.data || !imageData.width || !imageData.height) {
    throw new TypeError("Expected ImageData-compatible { data, width, height } input.");
  }
  return {
    data: imageData.data instanceof Uint8ClampedArray ? imageData.data : new Uint8ClampedArray(imageData.data),
    width: Number(imageData.width),
    height: Number(imageData.height)
  };
}

export function scanImageData(imageData, options = {}) {
  const source = ensureImageDataLike(imageData);
  return normalizeDecoded(wasm().scanRgba(source.data, source.width, source.height, wasmOptions(copyOptions(options))));
}

export function debugScanImageData(imageData, options = {}) {
  const source = ensureImageDataLike(imageData);
  const result = wasm().debugScanRgba(source.data, source.width, source.height, wasmOptions(copyOptions(options)));
  if (result?.result) normalizeDecoded(result.result);
  return result;
}

export function rectifyDetectedCode(imageData, options = {}) {
  const source = ensureImageDataLike(imageData);
  const result = wasm().rectifyRgba(
    source.data,
    source.width,
    source.height,
    wasmOptions(copyOptions(options)),
    Number(options.moduleSize ?? 10)
  );
  if (result?.imageData?.data) result.imageData.data = new Uint8ClampedArray(result.imageData.data);
  return result;
}

function videoFrameToImageData(video, options = {}) {
  if (!video?.videoWidth || !video?.videoHeight) throw new Error("Video frame is not ready.");
  const maxDimension = Math.max(64, Number(options.maxDimension ?? 960));
  const scale = Math.min(1, maxDimension / Math.max(video.videoWidth, video.videoHeight));
  const width = Math.max(1, Math.round(video.videoWidth * scale));
  const height = Math.max(1, Math.round(video.videoHeight * scale));
  const canvas = typeof OffscreenCanvas !== "undefined"
    ? new OffscreenCanvas(width, height)
    : Object.assign(document.createElement("canvas"), { width, height });
  const ctx = canvas.getContext("2d", { willReadFrequently: true, alpha: false });
  ctx.drawImage(video, 0, 0, width, height);
  return ctx.getImageData(0, 0, width, height);
}

export function scanVideoFrame(video, options = {}) {
  return scanImageData(videoFrameToImageData(video, options), options);
}

export async function scanFile(file, options = {}) {
  if (!file) throw new Error("scanFile requires a File or Blob.");
  let bitmap = null;
  let url = null;
  try {
    if (typeof createImageBitmap === "function") bitmap = await createImageBitmap(file);
    if (!bitmap) {
      url = URL.createObjectURL(file);
      bitmap = await new Promise((resolve, reject) => {
        const image = new Image();
        image.onload = () => resolve(image);
        image.onerror = () => reject(new Error("Unable to decode image file."));
        image.src = url;
      });
    }
    const sourceWidth = bitmap.naturalWidth ?? bitmap.width;
    const sourceHeight = bitmap.naturalHeight ?? bitmap.height;
    const maxDimension = Math.max(64, Number(options.maxDimension ?? 1800));
    const scale = Math.min(1, maxDimension / Math.max(sourceWidth, sourceHeight));
    const width = Math.max(1, Math.round(sourceWidth * scale));
    const height = Math.max(1, Math.round(sourceHeight * scale));
    const canvas = typeof OffscreenCanvas !== "undefined"
      ? new OffscreenCanvas(width, height)
      : Object.assign(document.createElement("canvas"), { width, height });
    const ctx = canvas.getContext("2d", { willReadFrequently: true, alpha: false });
    ctx.drawImage(bitmap, 0, 0, width, height);
    return scanImageData(ctx.getImageData(0, 0, width, height), options);
  } finally {
    bitmap?.close?.();
    if (url) URL.revokeObjectURL(url);
  }
}

function paletteFor(options = {}) {
  const mode = options.mode === "print" ? "print" : "screen";
  const base = mode === "print" ? PRINT_PALETTE : DEFAULT_PALETTE;
  return { ...base, ...(options.palette ?? {}) };
}

function colorToRgb(value) {
  if (Array.isArray(value)) return value.slice(0, 3);
  const text = String(value);
  const match = /^#?([0-9a-f]{6})$/i.exec(text);
  if (!match) throw new Error(`Invalid RGB color: ${text}`);
  const n = parseInt(match[1], 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

function paletteRgb(options = {}) {
  const palette = paletteFor(options);
  return {
    black: colorToRgb(palette.black), white: colorToRgb(palette.white), red: colorToRgb(palette.red),
    green: colorToRgb(palette.green), blue: colorToRgb(palette.blue)
  };
}

function cellRgb(cell, palette) {
  if (cell === CELL.BLACK) return palette.black;
  if (cell === CELL.RED) return palette.red;
  if (cell === CELL.GREEN) return palette.green;
  if (cell === CELL.BLUE) return palette.blue;
  return palette.white;
}

function cellHex(cell, palette) {
  if (cell === CELL.BLACK) return palette.black;
  if (cell === CELL.RED) return palette.red;
  if (cell === CELL.GREEN) return palette.green;
  if (cell === CELL.BLUE) return palette.blue;
  return palette.white;
}

function trianglePair(cell) {
  const value = Number(cell);
  if (!Number.isInteger(value) || value < 0 || value > 15) throw new Error(`Invalid Triangle16 cell ${cell}.`);
  return [(value >> 2) & 3, value & 3];
}

function renderPlan(codeOrMatrix, options = {}) {
  const code = Array.isArray(codeOrMatrix) ? { matrix: codeOrMatrix } : codeOrMatrix;
  if (!code?.matrix?.length) throw new Error("Expected an encoded QuadQR object or matrix.");
  const matrix = code.matrix;
  const size = matrix.length;
  const version = Number(code.version ?? ((size - 21) / 4 + 1));
  const quietZone = Math.max(0, Number(options.quietZone ?? 4));
  const totalModules = size + quietZone * 2;
  const imageSize = options.imageSize != null
    ? Math.max(totalModules, Math.round(Number(options.imageSize)))
    : options.moduleSize != null
      ? totalModules * Math.max(1, Math.floor(Number(options.moduleSize)))
      : Math.max(totalModules, 720);
  const style = options.mode === "print" && !options.allowStyledPrint ? "classic" : (options.style ?? "classic");
  const highDensity = Boolean(code.highDensity ?? code.cellEncoding === "triangle16");
  const reserved = wasm().layoutReserved(version);
  return { code, matrix, size, version, quietZone, totalModules, imageSize, style, highDensity, reserved };
}

function moduleBounds(index, quietZone, totalModules, imageSize) {
  const a = Math.round((index + quietZone) * imageSize / totalModules);
  const b = Math.round((index + quietZone + 1) * imageSize / totalModules);
  return [a, b];
}

function renderBasePixels(codeOrMatrix, options = {}) {
  const plan = renderPlan(codeOrMatrix, options);
  const palette = paletteRgb(options);
  const width = plan.imageSize;
  const height = plan.imageSize;
  const data = new Uint8ClampedArray(width * height * 4);
  const white = palette.white;
  for (let i = 0; i < width * height; i++) {
    const offset = i * 4;
    data[offset] = white[0]; data[offset + 1] = white[1]; data[offset + 2] = white[2]; data[offset + 3] = 255;
  }
  const put = (x, y, rgb) => {
    if (x < 0 || y < 0 || x >= width || y >= height) return;
    const offset = (y * width + x) * 4;
    data[offset] = rgb[0]; data[offset + 1] = rgb[1]; data[offset + 2] = rgb[2]; data[offset + 3] = 255;
  };

  for (let row = 0; row < plan.size; row++) {
    const [y0, y1] = moduleBounds(row, plan.quietZone, plan.totalModules, width);
    for (let col = 0; col < plan.size; col++) {
      const [x0, x1] = moduleBounds(col, plan.quietZone, plan.totalModules, width);
      const cell = plan.matrix[row][col];
      const structural = Boolean(plan.reserved?.[row]?.[col]) || cell === CELL.BLACK;
      if (plan.highDensity && !structural) {
        const [first, second] = trianglePair(cell);
        const firstRgb = cellRgb(first, palette);
        const secondRgb = cellRgb(second, palette);
        const w = Math.max(1, x1 - x0), h = Math.max(1, y1 - y0);
        for (let y = y0; y < y1; y++) {
          for (let x = x0; x < x1; x++) {
            const u = (x - x0 + 0.5) / w;
            const v = (y - y0 + 0.5) / h;
            put(x, y, u + v <= 1 ? firstRgb : secondRgb);
          }
        }
        continue;
      }
      let rgb = cellRgb(cell > 3 ? CELL.WHITE : cell, palette);
      let inset = 0;
      let radius = 0;
      if (!structural && plan.style === "depth") {
        const hash = (((row + 1) * 2654435761) ^ ((col + 1) * 2246822507) ^ ((cell + 2) * 3266489909)) >>> 0;
        const t = (hash & 0xffff) / 0xffff;
        const opacity = t < .16 ? .80 : t < .36 ? .87 : t < .56 ? .94 : 1;
        rgb = rgb.map((c, i) => Math.round(white[i] + (c - white[i]) * opacity));
      } else if (!structural && plan.style === "soft") {
        inset = Math.max(0, Math.floor(Math.min(x1 - x0, y1 - y0) * .07));
        radius = Math.max(1, Math.floor(Math.min(x1 - x0, y1 - y0) * .20));
      }
      for (let y = y0 + inset; y < y1 - inset; y++) {
        for (let x = x0 + inset; x < x1 - inset; x++) {
          if (radius > 0) {
            const left = x0 + inset, right = x1 - inset - 1, top = y0 + inset, bottom = y1 - inset - 1;
            const cx = x < left + radius ? left + radius : x > right - radius ? right - radius : x;
            const cy = y < top + radius ? top + radius : y > bottom - radius ? bottom - radius : y;
            if ((x - cx) ** 2 + (y - cy) ** 2 > radius ** 2) continue;
          }
          put(x, y, rgb);
        }
      }
    }
  }
  return { ...plan, data, width, height };
}

function logoSourceDimensions(source) {
  const width = Number(source?.naturalWidth ?? source?.videoWidth ?? source?.width);
  const height = Number(source?.naturalHeight ?? source?.videoHeight ?? source?.height);
  return width > 0 && height > 0 ? { width, height } : null;
}

function containRect(x, y, size, dimensions) {
  if (!dimensions) return { x, y, width: size, height: size };
  const scale = Math.min(size / dimensions.width, size / dimensions.height);
  const width = dimensions.width * scale;
  const height = dimensions.height * scale;
  return { x: x + (size - width) / 2, y: y + (size - height) / 2, width, height };
}

function logoGeometry(code, options, imageSize) {
  const matrixSize = Number(code?.size ?? code?.matrix?.length ?? 0);
  const quietZone = Math.max(0, Number(options.quietZone ?? 4));
  const totalModules = matrixSize + quietZone * 2;
  const modulePx = imageSize / totalModules;
  const ratio = logoRatio(code, options);
  const side = matrixSize * modulePx * ratio;
  const x = (imageSize - side) / 2;
  const y = x;
  const logo = options.logo;
  const padding = Number(logo?.padding ?? options.logoPadding ?? .65) * modulePx;
  const radius = Number(logo?.radius ?? options.logoRadius ?? .8) * modulePx;
  return { ratio, side, x, y, padding, radius, modulePx };
}

function blendLogoPixel(data, width, x, y, rgb, alpha) {
  const height = data.length / 4 / width;
  if (x < 0 || y < 0 || x >= width || y >= height) return;
  const p = (y * width + x) * 4;
  const a = Math.max(0, Math.min(1, alpha));
  data[p] = Math.round(data[p] * (1 - a) + rgb[0] * a);
  data[p + 1] = Math.round(data[p + 1] * (1 - a) + rgb[1] * a);
  data[p + 2] = Math.round(data[p + 2] * (1 - a) + rgb[2] * a);
  data[p + 3] = 255;
}

function drawLogoToImageData(code, rendered, options = {}) {
  const logo = options.logo;
  const source = logo?.source ?? logo;
  if (!source) return;
  if (!source?.data || !Number.isInteger(source.width) || !Number.isInteger(source.height)) {
    throw new Error("renderToImageData() logo must be an ImageData-like object with width, height, and RGBA data.");
  }
  const geometry = logoGeometry(code, options, rendered.width);
  const palette = paletteRgb(options);
  if (logo?.clearBackground ?? options.logoClearBackground) {
    const x0 = Math.max(0, Math.floor(geometry.x - geometry.padding));
    const y0 = Math.max(0, Math.floor(geometry.y - geometry.padding));
    const x1 = Math.min(rendered.width, Math.ceil(geometry.x + geometry.side + geometry.padding));
    const y1 = Math.min(rendered.height, Math.ceil(geometry.y + geometry.side + geometry.padding));
    for (let y = y0; y < y1; y++) {
      for (let x = x0; x < x1; x++) blendLogoPixel(rendered.data, rendered.width, x, y, palette.white, 1);
    }
  }
  const dest = containRect(geometry.x, geometry.y, geometry.side, logoSourceDimensions(source));
  const destWidth = Math.max(1, Math.round(dest.width));
  const destHeight = Math.max(1, Math.round(dest.height));
  const x0 = Math.round(dest.x);
  const y0 = Math.round(dest.y);
  for (let y = 0; y < destHeight; y++) {
    const sy = Math.min(source.height - 1, Math.floor((y + .5) * source.height / destHeight));
    for (let x = 0; x < destWidth; x++) {
      const sx = Math.min(source.width - 1, Math.floor((x + .5) * source.width / destWidth));
      const p = (sy * source.width + sx) * 4;
      const alpha = Number(source.data[p + 3] ?? 255) / 255;
      if (alpha <= 0) continue;
      blendLogoPixel(rendered.data, rendered.width, x0 + x, y0 + y, [source.data[p], source.data[p + 1], source.data[p + 2]], alpha);
    }
  }
}

export function renderToImageData(codeOrMatrix, options = {}) {
  const rendered = renderBasePixels(codeOrMatrix, options);
  if (!Array.isArray(codeOrMatrix) && options.logo) drawLogoToImageData(codeOrMatrix, rendered, options);
  if (typeof ImageData !== "undefined") return new ImageData(rendered.data, rendered.width, rendered.height);
  return { data: rendered.data, width: rendered.width, height: rendered.height };
}

function logoRatio(code, options = {}) {
  const value = options.logo?.size ?? options.logoSize;
  if (value === "auto" || value == null) return estimateSafeLogoSize(code, options);
  return Math.max(.05, Math.min(.30, Number(value)));
}

function drawLogoToCanvas(code, canvas, options = {}) {
  const logo = options.logo;
  const source = logo?.source ?? logo;
  if (!source || typeof canvas.getContext !== "function") return;
  const ctx = canvas.getContext("2d");
  const geometry = logoGeometry(code, options, canvas.width);
  const { side, x, y, padding } = geometry;
  if (logo.clearBackground) {
    ctx.fillStyle = paletteFor(options).white;
    ctx.fillRect(x - padding, y - padding, side + padding * 2, side + padding * 2);
  }
  ctx.save();
  const radius = geometry.radius;
  if (radius > 0) {
    ctx.beginPath();
    ctx.roundRect(x, y, side, side, Math.min(radius, side / 2));
    ctx.clip();
  }
  ctx.drawImage(source, x, y, side, side);
  ctx.restore();
}

export function renderToCanvas(codeOrMatrix, canvas, options = {}) {
  if (!canvas?.getContext) throw new Error("renderToCanvas requires a canvas element.");
  const imageData = renderToImageData(codeOrMatrix, { ...options, logo: undefined });
  canvas.width = imageData.width;
  canvas.height = imageData.height;
  const ctx = canvas.getContext("2d", { alpha: false, willReadFrequently: true });
  const data = imageData instanceof ImageData ? imageData : new ImageData(imageData.data, imageData.width, imageData.height);
  ctx.putImageData(data, 0, 0);
  if (!Array.isArray(codeOrMatrix) && options.logo) drawLogoToCanvas(codeOrMatrix, canvas, options);
  return canvas;
}

function xmlEscape(value) {
  return String(value).replace(/[&<>\"]/g, ch => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[ch]);
}

export function renderToSVG(codeOrMatrix, options = {}) {
  const plan = renderPlan(codeOrMatrix, options);
  const palette = paletteFor(options);
  const size = plan.imageSize;
  const parts = [`<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 ${size} ${size}" shape-rendering="crispEdges">`, `<rect width="100%" height="100%" fill="${xmlEscape(palette.white)}"/>`];
  for (let row = 0; row < plan.size; row++) {
    const [y0, y1] = moduleBounds(row, plan.quietZone, plan.totalModules, size);
    for (let col = 0; col < plan.size; col++) {
      const [x0, x1] = moduleBounds(col, plan.quietZone, plan.totalModules, size);
      const cell = plan.matrix[row][col];
      const structural = Boolean(plan.reserved?.[row]?.[col]) || cell === CELL.BLACK;
      if (plan.highDensity && !structural) {
        const [first, second] = trianglePair(cell);
        const a = cellHex(first, palette), b = cellHex(second, palette);
        parts.push(`<path d="M${x0},${y0} H${x1} L${x0},${y1} Z" fill="${a}"/>`);
        parts.push(`<path d="M${x1},${y1} H${x0} L${x1},${y0} Z" fill="${b}"/>`);
        continue;
      }
      const color = cellHex(cell > 3 ? CELL.WHITE : cell, palette);
      if (color.toLowerCase() === String(palette.white).toLowerCase()) continue;
      const w = x1 - x0, h = y1 - y0;
      if (!structural && plan.style === "soft") {
        const inset = Math.max(0, Math.floor(Math.min(w, h) * .07));
        const radius = Math.max(1, Math.floor(Math.min(w, h) * .20));
        parts.push(`<rect x="${x0 + inset}" y="${y0 + inset}" width="${Math.max(1, w - 2 * inset)}" height="${Math.max(1, h - 2 * inset)}" rx="${radius}" fill="${color}"/>`);
      } else {
        parts.push(`<rect x="${x0}" y="${y0}" width="${w}" height="${h}" fill="${color}"/>`);
      }
    }
  }
  if (!Array.isArray(codeOrMatrix) && options.logo) {
    const source = options.logo.source;
    if (typeof source === "string") {
      const geometry = logoGeometry(codeOrMatrix, options, size);
      const { side, x, y, modulePx, padding } = geometry;
      if (options.logo.clearBackground) {
        parts.push(`<rect x="${x - padding}" y="${y - padding}" width="${side + padding * 2}" height="${side + padding * 2}" rx="${Number(options.logo.radius ?? .8) * modulePx}" fill="${xmlEscape(palette.white)}"/>`);
      }
      parts.push(`<image href="${xmlEscape(source)}" x="${x}" y="${y}" width="${side}" height="${side}" preserveAspectRatio="xMidYMid meet"/>`);
    }
  }
  parts.push("</svg>");
  return parts.join("");
}

export function estimateSafeLogoSize(codeOrMatrix, options = {}) {
  const code = Array.isArray(codeOrMatrix) ? null : codeOrMatrix;
  const ecc = String(code?.eccLevel ?? options.ecc ?? DEFAULT_ECC_LEVEL).toUpperCase();
  const base = { L: .10, M: .13, Q: .16, H: .19 }[ecc] ?? .12;
  const utilization = Number.isFinite(code?.utilization) ? code.utilization : Number(options.utilization ?? .65);
  const utilizationPenalty = utilization <= .55 ? 1 : Math.max(.72, 1 - (utilization - .55) * .56);
  const clearBackground = Boolean(options.clearBackground ?? options.logoClearBackground ?? options.logo?.clearBackground);
  const clearPenalty = clearBackground ? .90 : 1;
  const mode = String(options.mode ?? options.renderMode ?? RENDER_MODES.SCREEN).toLowerCase();
  const printPenalty = mode === RENDER_MODES.PRINT ? .88 : 1;
  const versionPenalty = code?.version === 1 ? .88 : 1;
  return Math.max(.07, Math.min(.22, base * utilizationPenalty * clearPenalty * printPenalty * versionPenalty));
}

export function findMaxSafeLogoSize(code, options = {}) {
  if (!code?.matrix) throw new Error("findMaxSafeLogoSize expects an encoded QuadQR object.");
  const logo = options.logo;
  const source = logo?.source ?? logo;
  if (!source?.data || !source?.width || !source?.height) {
    throw new Error("findMaxSafeLogoSize requires an ImageData-like logo source so it can test pixel output without DOM dependencies.");
  }
  let low = Math.max(.05, Number(options.minSize ?? .05));
  let high = Math.min(.30, Number(options.maxSize ?? .28));
  let best = 0;
  const iterations = Math.max(3, Math.min(10, Math.floor(options.iterations ?? 7)));
  for (let i = 0; i < iterations; i++) {
    const probe = (low + high) / 2;
    try {
      const image = renderToImageData(code, {
        ...options,
        logo: {
          ...(typeof logo === "object" && Object.prototype.hasOwnProperty.call(logo, "source") ? logo : { source }),
          size: probe
        }
      });
      const decoded = scanImageData(image, { minVersion: code.version, maxVersion: code.version });
      if (decoded.crc32 !== code.crc32) throw new Error("Decoded a different payload.");
      best = probe;
      low = probe;
    } catch {
      high = probe;
    }
  }
  return {
    safeSize: best || estimateSafeLogoSize(code, options),
    testedMax: Number(options.maxSize ?? .28),
    iterations,
    empirical: best > 0
  };
}

export function getPrintGuidance(codeOrMatrix, options = {}) {
  const matrix = Array.isArray(codeOrMatrix) ? codeOrMatrix : codeOrMatrix?.matrix;
  if (!matrix?.length) throw new Error("getPrintGuidance expects a QuadQR code or matrix.");
  const quietZone = Math.max(4, Math.floor(options.quietZone ?? 4));
  const physicalSizeMm = Number(options.physicalSizeMm ?? 35);
  const totalModules = matrix.length + quietZone * 2;
  const moduleSizeMm = physicalSizeMm / totalModules;
  const dpi = Number(options.dpi ?? 300);
  const modulePixelsAtDpi = moduleSizeMm / 25.4 * dpi;
  const recommendedMinimumModuleMm = Number(options.minimumModuleMm ?? .40);
  return {
    mode: "print",
    physicalSizeMm,
    quietZone,
    moduleSizeMm,
    modulePixelsAtDpi,
    dpi,
    recommendedMinimumModuleMm,
    meetsRecommendedModuleSize: moduleSizeMm >= recommendedMinimumModuleMm,
    recommendedMinimumPhysicalSizeMm: totalModules * recommendedMinimumModuleMm,
    palette: PRINT_PALETTE,
    recommendations: [
      "Use classic rendering for print unless a styled output has been physically validated.",
      "Keep at least a 4-module quiet zone.",
      `Target at least ${recommendedMinimumModuleMm.toFixed(2)} mm per module for general-purpose printing.`,
      "Avoid glossy reflections and low-ink/toner modes when reliable color classification matters."
    ]
  };
}

export function applyStressDistortion(imageData, type, severity = 0.5, options = {}) {
  const source = ensureImageDataLike(imageData);
  const result = wasm().applyStressDistortion(source.data, source.width, source.height, String(type), Number(severity), wasmOptions(copyOptions(options)));
  return { width: result.width, height: result.height, data: new Uint8ClampedArray(result.data) };
}

export function runImageStressTest(imageData, expected = {}, options = {}) {
  const source = ensureImageDataLike(imageData);
  return wasm().runImageStressTest(source.data, source.width, source.height, Number(expected.crc32 ?? 0), wasmOptions(copyOptions(options)));
}

export function runReliabilityLab(imageData, expected = {}, options = {}) {
  const source = ensureImageDataLike(imageData);
  return wasm().runReliabilityLab(source.data, source.width, source.height, Number(expected.crc32 ?? 0), wasmOptions(copyOptions(options)));
}

export function runPerspectiveSweep(imageData, expected = {}, options = {}) {
  const source = ensureImageDataLike(imageData);
  const axis = options.axis ?? "yaw";
  const angles = options.angles ?? (axis === "roll" ? [0, 20, 35, 50, 65, 75] : [0, 15, 25, 35, 45, 55]);
  return wasm().runPerspectiveSweep(source.data, source.width, source.height, Number(expected.crc32 ?? 0), axis, angles, wasmOptions(copyOptions(options)));
}

/** Render the code and run the standard scanability torture suite. */
export function assessScanability(code, renderOptions = {}, options = {}) {
  if (!code?.matrix) throw new Error("assessScanability expects an encoded QuadQR object.");
  const testImageSize = Math.max(240, Math.min(Number(options.testImageSize ?? 480), Number(renderOptions.imageSize ?? 720)));
  const image = renderToImageData(code, { ...renderOptions, imageSize: testImageSize });
  const report = runImageStressTest(image, { version: code.version, crc32: code.crc32 }, options);
  const autoLogoSize = renderOptions.logo ? estimateSafeLogoSize(code, renderOptions) : null;
  return {
    ...report,
    version: code.version,
    eccLevel: code.eccLevel,
    utilization: code.utilization,
    testImageSize,
    autoLogoSize,
    recommendations: [
      ...(report.score < 75 ? ["Increase ECC, reduce logo size, use Classic style, or increase output/physical size."] : []),
      ...(Number(renderOptions.quietZone ?? 4) < 4 ? ["Use a quiet zone of at least 4 modules for robust camera/print scanning."] : []),
      ...(renderOptions.logo && autoLogoSize != null ? [`Auto-safe logo estimate: ${(autoLogoSize * 100).toFixed(1)}% of the symbol width.`] : []),
      ...(String(renderOptions.mode ?? renderOptions.renderMode ?? "screen").toLowerCase() === "print" ? getPrintGuidance(code, renderOptions).recommendations : [])
    ]
  };
}

export async function startCameraScanner(video, options = {}) {
  if (!video) throw new Error("startCameraScanner requires a video element.");
  if (!navigator.mediaDevices?.getUserMedia) throw new Error("Camera access is unavailable in this browser.");
  if (options.signal?.aborted) throw new DOMException("Camera scanner aborted.", "AbortError");

  const stream = await navigator.mediaDevices.getUserMedia(
    options.constraints ?? {
      video: options.videoConstraints ?? { facingMode: { ideal: "environment" } },
      audio: false
    }
  );
  video.srcObject = stream;
  video.playsInline = true;
  video.muted = true;
  await video.play();

  const workerUrl = options.cameraWorkerUrl
    ? new URL(options.cameraWorkerUrl, document.baseURI)
    : new URL("./camera-scanner-worker.js", import.meta.url);
  const scanInterval = Math.max(16, Number(options.scanInterval ?? 33));
  const maxDimension = Math.max(160, Number(options.maxDimension ?? 960));
  const stopOnResult = options.stopOnResult !== false;
  const workerReadyTimeoutMs = Math.max(1500, Number(options.cameraWorkerReadyTimeoutMs ?? 8000));

  let stopped = false;
  let paused = false;
  let inFlight = false;
  let frame = 0;
  let lastScanAt = -Infinity;
  let rafId = 0;
  let videoFrameHandle = 0;
  let visibilityHandler = null;
  let abortHandler = null;
  let worker = null;
  let workerReady = false;
  let workerReadyTimer = null;
  let singleFallbackStarted = false;

  const scratch = options.canvas ?? document.createElement("canvas");
  const ctx = scratch.getContext("2d", { willReadFrequently: true, alpha: false });
  if (!ctx) throw new Error("Unable to create camera scan canvas.");

  const emitCameraState = (state, extra = {}) => {
    try { options.onCameraState?.({ state, timestamp: Date.now(), ...extra }); } catch {}
  };

  const emitDiagnostic = (event) => {
    try { options.onDiagnostic?.(event); } catch {}
  };

  const cancelScheduledScan = () => {
    if (rafId) cancelAnimationFrame(rafId);
    if (videoFrameHandle && video.cancelVideoFrameCallback) {
      try { video.cancelVideoFrameCallback(videoFrameHandle); } catch {}
    }
    rafId = 0;
    videoFrameHandle = 0;
  };

  const clearWorkerReadyTimer = () => {
    if (workerReadyTimer) clearTimeout(workerReadyTimer);
    workerReadyTimer = null;
  };

  const stopTracks = () => {
    for (const track of stream.getTracks()) track.stop();
    if (video.srcObject === stream) video.srcObject = null;
  };

  const serializableOptions = () => {
    const out = { ...options };
    for (const key of [
      "canvas", "constraints", "videoConstraints", "onDecode", "onDiagnostic",
      "onResult", "onScanMiss", "onCameraState", "signal", "cameraWorkerUrl",
      "maxDimension", "scanInterval", "cameraWorkerReadyTimeoutMs", "wasmThreads",
      "threadCount"
    ]) delete out[key];
    for (const [key, value] of Object.entries(out)) {
      if (typeof value === "function" || value == null) delete out[key];
    }
    return out;
  };

  const sendCurrentFrame = (force = false) => {
    if (stopped || !workerReady || !worker || (!force && paused) || inFlight || video.readyState < 2) return false;
    const sourceWidth = video.videoWidth;
    const sourceHeight = video.videoHeight;
    if (!sourceWidth || !sourceHeight) return false;
    const scale = Math.min(1, maxDimension / Math.max(sourceWidth, sourceHeight));
    const width = Math.max(1, Math.round(sourceWidth * scale));
    const height = Math.max(1, Math.round(sourceHeight * scale));
    if (scratch.width !== width || scratch.height !== height) {
      scratch.width = width;
      scratch.height = height;
    }
    ctx.drawImage(video, 0, 0, width, height);
    const imageData = ctx.getImageData(0, 0, width, height);
    const nextFrame = ++frame;
    inFlight = true;
    worker.postMessage({
      type: "scan",
      frame: nextFrame,
      width,
      height,
      rgba: imageData.data,
      options: serializableOptions()
    }, [imageData.data.buffer]);
    return true;
  };

  const scheduleNextScan = () => {
    if (stopped || paused || !workerReady) return;
    if (video.requestVideoFrameCallback) videoFrameHandle = video.requestVideoFrameCallback(capture);
    else rafId = requestAnimationFrame(capture);
  };

  const capture = timestamp => {
    if (stopped || paused || !workerReady) return;
    const now = Number(timestamp ?? performance.now());
    if (!inFlight && now - lastScanAt >= scanInterval) {
      lastScanAt = now;
      sendCurrentFrame();
    }
    scheduleNextScan();
  };

  const controller = {
    stream,
    stop() {
      if (stopped) return;
      stopped = true;
      paused = false;
      workerReady = false;
      clearWorkerReadyTimer();
      cancelScheduledScan();
      worker?.terminate();
      worker = null;
      stopTracks();
      if (visibilityHandler) document.removeEventListener("visibilitychange", visibilityHandler);
      if (abortHandler && options.signal?.removeEventListener) options.signal.removeEventListener("abort", abortHandler);
      emitCameraState("stopped");
    },
    pause() {
      if (stopped || paused) return;
      paused = true;
      cancelScheduledScan();
      emitDiagnostic({ type: "camera-paused", state: "paused", method: "camera", message: "Camera scanning paused" });
      emitCameraState("paused");
    },
    resume() {
      if (stopped || !paused) return;
      paused = false;
      lastScanAt = -Infinity;
      emitDiagnostic({ type: "camera-resumed", state: "running", method: "camera", message: "Camera scanning resumed with fresh state" });
      emitCameraState("running", { resumed: true });
      scheduleNextScan();
    },
    scanNow() {
      if (stopped) throw new Error("Camera scanner is stopped.");
      return sendCurrentFrame(true);
    },
    get video() { return video; },
    get paused() { return paused; },
    get continuous() { return !stopOnResult; },
    get wasm() { return getWasmState(); }
  };

  const startWorker = (threadPreference, reason = null) => {
    if (stopped) return;
    clearWorkerReadyTimer();
    cancelScheduledScan();
    inFlight = false;
    workerReady = false;
    worker?.terminate();

    const candidate = new Worker(workerUrl, { type: "module", name: "quadqr-camera-scanner" });
    worker = candidate;
    if (reason) {
      emitDiagnostic({ type: "method", state: "trying", method: "rust-wasm", message: reason });
    }

    let initSent = false;
    const armWorkerTimeout = stage => {
      clearWorkerReadyTimer();
      workerReadyTimer = setTimeout(() => {
        if (candidate !== worker || stopped || workerReady) return;
        if (threadPreference !== false && !singleFallbackStarted) {
          singleFallbackStarted = true;
          startWorker(false, stage === "boot"
            ? "Threaded scanner worker did not boot; retrying baseline WASM."
            : "Threaded scanner startup timed out; retrying baseline WASM.");
          return;
        }
        const detail = stage === "boot"
          ? "QuadQR scanner worker did not boot."
          : "QuadQR scanner worker did not become ready.";
        emitDiagnostic({ type: "method", state: "failed", method: "rust-wasm", message: detail });
        emitCameraState("error", { error: detail });
      }, workerReadyTimeoutMs);
    };

    candidate.addEventListener("message", event => {
      if (candidate !== worker || stopped) return;
      const message = event.data ?? {};
      if (message.type === "boot") {
        emitDiagnostic({
          type: "method",
          state: "trying",
          method: "rust-wasm-worker",
          message: `Scanner worker booted · baseline ${message.backend ?? "single"}`
        });
        // Critical handshake: module workers can spend time resolving imports
        // before their message listener exists. Sending init immediately after
        // new Worker() can therefore race startup. The worker announces boot
        // only after its listener is installed; respond with init here.
        if (!initSent) {
          initSent = true;
          candidate.postMessage({
            type: "init",
            threads: threadPreference,
            threadCount: options.threadCount ?? null
          });
          armWorkerTimeout("ready");
        }
        return;
      }
      if (message.type === "init-start") {
        emitDiagnostic({
          type: "method",
          state: "trying",
          method: "rust-wasm-init",
          message: message.threads === false ? "Initializing baseline Rust/WASM scanner" : "Initializing Rust/WASM scanner"
        });
        return;
      }
      if (message.type === "init-error") {
        clearWorkerReadyTimer();
        const detail = message.error?.message || "Camera scanner WASM initialization failed";
        emitDiagnostic({ type: "method", state: "failed", method: "rust-wasm", message: detail });
        emitCameraState("error", { error: detail });
        return;
      }
      if (message.type === "ready") {
        clearWorkerReadyTimer();
        workerReady = true;
        emitDiagnostic({
          type: "camera-ready",
          state: "ready",
          message: `Rust/WASM scanner ready · ${message.backend}${message.threads > 1 ? ` · ${message.threads} threads` : ""}${message.warning ? ` · fallback: ${message.warning}` : ""}`
        });
        emitCameraState(paused ? "paused" : "running", { ready: true, wasm: message });
        if (!paused) scheduleNextScan();
        return;
      }
      if (message.type !== "scan-result") return;
      inFlight = false;
      if (message.ok) {
        const result = normalizeDecoded(message.result);
        const imageData = message.rgba
          ? { width: message.width, height: message.height, data: new Uint8ClampedArray(message.rgba) }
          : null;
        const diagnostic = result.diagnostics ?? null;
        emitDiagnostic({ type: "success", state: "decoded", method: result.scanPath ?? "rust-wasm", message: `Decoded QuadQR v${result.version}` });
        options.onDecode?.(result, { frame: message.frame, imageData, diagnostic });
        options.onResult?.(result, { frame: message.frame, imageData, diagnostic });
        if (stopOnResult) controller.stop();
      } else {
        if (message.diagnostic) emitDiagnostic(message.diagnostic);
        options.onScanMiss?.(message.error);
      }
    });

    candidate.addEventListener("error", event => {
      if (candidate !== worker || stopped) return;
      inFlight = false;
      const message = event.message || "Camera scanner worker failed";
      if (threadPreference !== false && !singleFallbackStarted) {
        singleFallbackStarted = true;
        startWorker(false, `Threaded scanner worker failed (${message}); retrying baseline WASM.`);
        return;
      }
      emitDiagnostic({ type: "method", state: "failed", method: "rust-wasm", message });
      emitCameraState("error", { error: message });
    });

    armWorkerTimeout("boot");
  };

  if (options.pauseWhenHidden !== false) {
    visibilityHandler = () => {
      if (document.hidden) controller.pause();
      else controller.resume();
    };
    document.addEventListener("visibilitychange", visibilityHandler);
    if (document.hidden) paused = true;
  }
  if (options.signal?.addEventListener) {
    abortHandler = () => controller.stop();
    options.signal.addEventListener("abort", abortHandler, { once: true });
  }

  emitCameraState("starting");
  startWorker(options.wasmThreads ?? false);
  return controller;
}

export const internals = Object.freeze({
  sizeForVersion(version) { return 21 + 4 * (Number(version) - 1); },
  versionFromSize(size) {
    const value = (Number(size) - 21) / 4 + 1;
    return Number.isInteger(value) && value >= 1 && value <= 40 ? value : null;
  },
  wasmState: getWasmState
});
