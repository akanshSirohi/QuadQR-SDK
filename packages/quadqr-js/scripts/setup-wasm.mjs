import { spawnSync } from "node:child_process";

function run(command, args, options = {}) {
  console.log(`\n> ${command} ${args.join(" ")}`);
  const result = spawnSync(command, args, {
    stdio: "inherit",
    shell: false,
    ...options
  });
  if (result.status !== 0) process.exit(result.status ?? 1);
}

function exists(command) {
  const probe = spawnSync(command, ["--version"], {
    stdio: "ignore",
    shell: false
  });
  return probe.status === 0;
}

if (!exists("rustup") || !exists("cargo")) {
  console.error("Rust is required. Install rustup from https://rustup.rs/ and run this command again.");
  process.exit(1);
}

const nightly = process.env.QUADQR_WASM_NIGHTLY ?? "nightly-2025-11-15";

run("rustup", ["target", "add", "wasm32-unknown-unknown"]);
run("rustup", ["toolchain", "install", nightly, "--component", "rust-src"]);
run("rustup", ["target", "add", "wasm32-unknown-unknown", "--toolchain", nightly]);

function wasmPackVersion() {
  const probe = spawnSync("wasm-pack", ["--version"], {
    encoding: "utf8",
    shell: false
  });
  if (probe.status !== 0) return null;
  const match = String(probe.stdout ?? probe.stderr ?? "").match(/wasm-pack\s+(\d+)\.(\d+)\.(\d+)/i);
  return match ? match.slice(1, 4).map(Number) : null;
}

function versionAtLeast(version, required) {
  for (let i = 0; i < 3; i++) {
    if (version[i] > required[i]) return true;
    if (version[i] < required[i]) return false;
  }
  return true;
}

const requiredWasmPack = [0, 15, 0];
const installedWasmPack = wasmPackVersion();
if (!installedWasmPack || !versionAtLeast(installedWasmPack, requiredWasmPack)) {
  console.log("QuadQR requires wasm-pack 0.15.0+ so Cargo target triples and JS output targets remain unambiguous.");
  run("cargo", ["install", "wasm-pack", "--version", "0.15.0", "--locked", "--force"]);
}

run("wasm-pack", ["--version"]);
console.log("\nQuadQR WASM toolchains are ready.");
