import { readFileSync, readdirSync, statSync, existsSync } from "node:fs";
import { resolve, dirname, extname } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const files = [];
function walk(dir) {
  for (const name of readdirSync(dir)) {
    if (["node_modules", "wasm"].includes(name)) continue;
    const path = resolve(dir, name);
    const stat = statSync(path);
    if (stat.isDirectory()) walk(path);
    else if ([".js", ".mjs"].includes(extname(path))) files.push(path);
  }
}
walk(root);
for (const file of files) {
  const result = spawnSync(process.execPath, ["--check", file], { stdio: "inherit" });
  if (result.status !== 0) process.exit(result.status ?? 1);
}

const packageJson = JSON.parse(readFileSync(resolve(root, "package.json"), "utf8"));
if (packageJson.name !== "quadqr-js") throw new Error("Unexpected package name.");
for (const required of ["library/quadqr.js", "library/wasm.js", "library/camera-scanner-worker.js", "demo/index.html", "demo/app.js", "demo/compute-worker.js"]) {
  if (!existsSync(resolve(root, required))) throw new Error(`Missing ${required}`);
}

const app = readFileSync(resolve(root, "demo/app.js"), "utf8");
const html = readFileSync(resolve(root, "demo/index.html"), "utf8");
const ids = [...app.matchAll(/querySelector\(\"#([^\"]+)/g)].map(match => match[1]);
const missing = [...new Set(ids)].filter(id => !html.includes(`id="${id}"`));
if (missing.length) throw new Error(`Demo HTML is missing IDs used by app.js: ${missing.join(", ")}`);

// Verify relative JavaScript imports resolve before the generated WASM folders exist.
for (const file of files) {
  const source = readFileSync(file, "utf8");
  for (const match of source.matchAll(/(?:from\s+|import\s*\()\s*["'](\.[^"']+)["']/g)) {
    const specifier = match[1].split("?")[0];
    const target = resolve(dirname(file), specifier);
    if (!existsSync(target) && !target.includes(`${String.raw`\\`}wasm${String.raw`\\`}`) && !target.includes("/wasm/")) {
      throw new Error(`Unresolved import ${specifier} from ${file}`);
    }
  }
}

// Every direct wasm().method() used by the compatibility wrapper must have a
// wasm-bindgen js_name export in the Rust bridge. This catches bridge typos even
// on machines where Rust/wasm-pack is not installed yet.
const quadqrSource = readFileSync(resolve(root, "library/quadqr.js"), "utf8");
const rustBridgePath = resolve(root, "../../crates/quadqr-wasm/src/lib.rs");
const rustBridge = readFileSync(rustBridgePath, "utf8");
const wasmCalls = [...quadqrSource.matchAll(/wasm\(\)\.([A-Za-z0-9_]+)/g)].map(match => match[1]);
const bindgenNames = new Set([...rustBridge.matchAll(/#\[wasm_bindgen\(js_name\s*=\s*([A-Za-z0-9_]+)\)\]/g)].map(match => match[1]));
const missingBindings = [...new Set(wasmCalls)].filter(name => !bindgenNames.has(name));
if (missingBindings.length) throw new Error(`JS wrapper calls missing Rust/WASM bindings: ${missingBindings.join(", ")}`);

const requiredExports = [
  "encodeText", "encodeBytes", "encodeUint8Array", "decodeMatrix", "decodeUint8Array",
  "encodeSignedText", "encodeSignedBytes", "encodeSecureText", "encodeSecureBytes", "decryptDecoded",
  "generateSigningKeyPair", "deriveSigningKeyId", "verifyDecodedSignature", "scanImageData",
  "debugScanImageData", "scanVideoFrame", "scanFile", "startCameraScanner", "renderToCanvas",
  "renderToImageData", "renderToSVG", "assessScanability", "runImageStressTest", "runReliabilityLab",
  "runPerspectiveSweep", "applyStressDistortion", "estimateSafeLogoSize", "findMaxSafeLogoSize",
  "getPrintGuidance", "compressPayload", "decompressPayload", "compressDeflatePayload",
  "decompressDeflatePayload", "compressBrotliPayload", "decompressBrotliPayload"
];
const missingExports = requiredExports.filter(name => !new RegExp(`export\\s+(?:async\\s+)?function\\s+${name}\\b`).test(quadqrSource));
if (missingExports.length) throw new Error(`Compatibility wrapper is missing public functions: ${missingExports.join(", ")}`);

const rootCargo = readFileSync(resolve(root, "../../Cargo.toml"), "utf8");
const coreCargo = readFileSync(resolve(root, "../../crates/quadqr/Cargo.toml"), "utf8");
if (!rootCargo.includes('"crates/quadqr-wasm"')) throw new Error("quadqr-wasm is not a workspace member.");
if (!/default\s*=\s*\[[^\]]*"parallel"/.test(coreCargo)) throw new Error("Native QuadQR must keep the parallel feature enabled by default.");

console.log(`Source validation passed for ${files.length} JavaScript files.`);
