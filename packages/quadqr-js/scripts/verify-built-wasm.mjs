import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const skipThreaded = /^(1|true|yes)$/i.test(process.env.QUADQR_SKIP_THREADED ?? "");

function requireFile(path, label) {
  if (!existsSync(path) || !statSync(path).isFile()) {
    throw new Error(`Missing ${label}: ${path}`);
  }
  if (statSync(path).size <= 0) throw new Error(`${label} is empty: ${path}`);
}

function requireText(path, patterns, label) {
  requireFile(path, label);
  const source = readFileSync(path, "utf8");
  for (const [pattern, description] of patterns) {
    if (!pattern.test(source)) throw new Error(`${label} does not expose ${description}.`);
  }
  return source;
}

const singleDir = resolve(root, "wasm/single");
const threadedDir = resolve(root, "wasm/threaded");
const nodeDir = resolve(root, "wasm/node");

for (const [dir, label] of [[singleDir, "single browser"], [nodeDir, "Node"]]) {
  requireFile(resolve(dir, "quadqr_wasm_bg.wasm"), `${label} WASM binary`);
}

requireText(resolve(singleDir, "quadqr_wasm.js"), [
  [/encodeText/, "encodeText"],
  [/scanRgba/, "scanRgba"],
  [/decodeMatrix/, "decodeMatrix"]
], "single browser WASM glue");

requireText(resolve(nodeDir, "quadqr_wasm.js"), [
  [/encodeText/, "encodeText"],
  [/scanRgba/, "scanRgba"],
  [/decodeMatrix/, "decodeMatrix"]
], "Node WASM glue");

if (!skipThreaded) {
  requireFile(resolve(threadedDir, "quadqr_wasm_bg.wasm"), "threaded WASM binary");
  requireText(resolve(threadedDir, "quadqr_wasm.js"), [
    [/encodeText/, "encodeText"],
    [/scanRgba/, "scanRgba"],
    [/initThreadPool/, "initThreadPool"]
  ], "threaded browser WASM glue");
} else {
  console.log("Threaded WASM verification skipped because QUADQR_SKIP_THREADED is enabled.");
}

console.log("Generated QuadQR WASM artifacts look complete.");
