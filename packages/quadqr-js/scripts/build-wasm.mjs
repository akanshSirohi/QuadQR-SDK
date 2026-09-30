import { spawnSync } from "node:child_process";
import { rmSync, mkdirSync, existsSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const pkgRoot = resolve(here, "..");
const repoRoot = resolve(pkgRoot, "../..");
const crate = resolve(repoRoot, "crates/quadqr-wasm");
const outputs = {
  single: resolve(pkgRoot, "wasm/single"),
  threaded: resolve(pkgRoot, "wasm/threaded"),
  node: resolve(pkgRoot, "wasm/node")
};

function commandExists(command) {
  const probe = spawnSync(command, ["--version"], {
    stdio: "ignore",
    shell: false
  });
  return probe.status === 0;
}

const WASM_RUST_TARGET = "wasm32-unknown-unknown";

function run(command, args, extraEnv = {}) {
  console.log(`\n> ${command} ${args.join(" ")}`);
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    stdio: "inherit",
    shell: false,
    env: {
      ...process.env,
      // wasm-pack's --target web/nodejs selects JavaScript glue output.
      // Cargo's target triple is separate and must always be wasm32-unknown-unknown.
      // Force it here so a machine-wide CARGO_BUILD_TARGET or Cargo config cannot
      // accidentally make wasm-pack ask rustup for a non-existent `web` target.
      CARGO_BUILD_TARGET: WASM_RUST_TARGET,
      ...extraEnv
    }
  });
  if (result.error) {
    console.error(`Failed to start ${command}:`, result.error.message);
    process.exit(1);
  }
  if (result.status !== 0) process.exit(result.status ?? 1);
}

if (!commandExists("wasm-pack")) {
  console.error("wasm-pack is not installed. Run `npm run setup:wasm` first.");
  process.exit(1);
}

const inheritedCargoTarget = process.env.CARGO_BUILD_TARGET;
if (inheritedCargoTarget && inheritedCargoTarget !== WASM_RUST_TARGET) {
  console.warn(
    `Ignoring inherited CARGO_BUILD_TARGET=${inheritedCargoTarget}; ` +
    `QuadQR WASM builds require ${WASM_RUST_TARGET}.`
  );
}

for (const dir of Object.values(outputs)) {
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
}

const common = [
  "build", crate,
  "--release",
  "--out-name", "quadqr_wasm",
  "--no-typescript"
];

// Baseline browser build. Stable Rust, no Rayon.
run("wasm-pack", [
  ...common,
  "--target", "web",
  "--out-dir", outputs.single,
  "--",
  "--target", WASM_RUST_TARGET
]);

// Node build. It intentionally stays single-threaded because browser-style
// SharedArrayBuffer worker bootstrapping is not used by the Node entry point.
run("wasm-pack", [
  ...common,
  "--target", "nodejs",
  "--out-dir", outputs.node,
  "--",
  "--target", WASM_RUST_TARGET
]);

const skipThreaded = /^(1|true|yes)$/i.test(process.env.QUADQR_SKIP_THREADED ?? "");
if (!skipThreaded) {
  if (!commandExists("rustup")) {
    console.error("rustup is required for the threaded WASM build. Run `npm run setup:wasm`.");
    process.exit(1);
  }

  // Follow the current upstream wasm-bindgen-rayon no-bundler configuration.
  // The newer nightly is also required by current QuadQR dependencies such as
  // image 0.25, whose recent releases have a newer MSRV than the older 2024
  // wasm-bindgen-rayon example toolchain.
  const nightly = process.env.QUADQR_WASM_NIGHTLY ?? "nightly-2025-11-15";
  const rustflags = [
    process.env.RUSTFLAGS,
    "-C target-feature=+atomics,+bulk-memory",
    "-C link-arg=--shared-memory",
    "-C link-arg=--max-memory=1073741824",
    "-C link-arg=--import-memory",
    "-C link-arg=--export=__wasm_init_tls",
    "-C link-arg=--export=__tls_size",
    "-C link-arg=--export=__tls_align",
    "-C link-arg=--export=__tls_base"
  ].filter(Boolean).join(" ");

  // Use rustup run rather than only setting RUSTUP_TOOLCHAIN. That guarantees
  // the cargo process spawned internally by wasm-pack receives the nightly
  // toolchain required for -Z build-std.
  run("rustup", [
    "run", nightly, "wasm-pack",
    ...common,
    "--target", "web",
    "--out-dir", outputs.threaded,
    "--",
    "--target", WASM_RUST_TARGET,
    "--features", "threads",
    "-Z", "build-std=panic_abort,std"
  ], { RUSTFLAGS: rustflags });
} else {
  writeFileSync(resolve(outputs.threaded, "README.txt"),
    "Threaded WASM build skipped because QUADQR_SKIP_THREADED was enabled.\n");
}

for (const [name, dir] of Object.entries(outputs)) {
  if (!existsSync(resolve(dir, "quadqr_wasm.js")) && name !== "threaded") {
    console.error(`Expected ${name} WASM glue was not generated.`);
    process.exit(1);
  }
}

console.log("\nQuadQR WASM builds complete.");
console.log(`  browser single: ${outputs.single}`);
console.log(`  browser threaded: ${outputs.threaded}${skipThreaded ? " (skipped)" : ""}`);
console.log(`  node: ${outputs.node}`);
