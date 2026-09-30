# quadqr-js Rust/WASM compatibility test package

This package explores the `quadqr-js` JavaScript API with the codec, compression, security and scanner engine backed by the Rust QuadQR implementation. It is experimental; the [original JavaScript package](https://github.com/akanshSirohi/QuadQR) remains the recommended web implementation.

The compatibility target is `quadqr-js` **1.5.7** at commit `0fdbab50c0ed4329017e201ef535c99730dc58ec`.

The package is marked `private` while this port is being validated so it cannot be published accidentally. Remove `private: true` only when you intentionally replace the npm package.

## One-time setup

From `packages/quadqr-js`:

```bash
npm run setup:wasm
```

This installs the normal `wasm32-unknown-unknown` target, the fixed threaded nightly toolchain with `rust-src`, and `wasm-pack` if it is missing.

## Run the full WASM demo

From the repository root on Windows:

```bat
run-wasm-demo.bat
```

On macOS/Linux:

```bash
./run-wasm-demo.sh
```

The one-click flow performs:

```text
static source validation
        ↓
single-thread browser WASM
        ↓
threaded Rayon browser WASM
        ↓
Node WASM
        ↓
generated-artifact validation
        ↓
API smoke tests
        ↓
COOP/COEP demo server
```

Then open:

```text
http://127.0.0.1:8080/demo/
```

## Manual commands

```bash
npm run sync:demo  # optional: refresh the pinned reference demo
npm run check:source
npm run build:wasm
npm run verify:wasm
npm run check
npm run serve
```

`npm run sync:demo` only refreshes `demo/styles.css` from the pinned QuadQR 1.5.7 commit. The adapted HTML, application, and worker files remain local.

## WASM backends

- `wasm/single`: stable Rust browser fallback without Rayon
- `wasm/threaded`: the same Rust scanner with Rayon + WebAssembly threads
- `wasm/node`: Node.js WASM build

The threaded build follows the current `wasm-bindgen-rayon` no-bundler setup. It uses a fixed nightly and rebuilds the wasm32 standard library with atomics/shared memory enabled.

## Camera performance model

JavaScript owns the camera and browser lifecycle. Decoding stays inside Rust/WASM.

```text
Camera
  ↓
newest available frame only
  ↓
dedicated camera Web Worker
  ↓
threaded WASM when supported
  ↓
Rust scan_rgba()
  ↓
Rayon finder/recovery/geometry work
```

Frames are not queued while a scan is running. This preserves the same design goal as the fast native Rust demo: parallelize work inside the newest frame, not multiple stale frames.

## Cross-origin isolation

True browser Rayon needs `SharedArrayBuffer`, which needs a cross-origin-isolated page.

The included local server sends:

```http
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

The synchronized demo also loads `coi-serviceworker.js` as a fallback for secure static hosts such as GitHub Pages where response headers cannot be configured directly.

If browser threading is unavailable, `initWasm()` automatically falls back to the single-thread WASM build without changing the public QuadQR API.

## Runtime selection

```js
await initWasm();                    // automatic
await initWasm({ threads: true });  // require threaded WASM
await initWasm({ threads: false }); // force baseline WASM
```

`startCameraScanner()` uses its own worker and requests the threaded backend automatically.

## Compatibility status

The ESM/browser API preserves the current names and call shapes used by the QuadQR demo, including encoding, matrix decoding, image/camera scanning, rendering, compression, security, Ed25519 signing, print guidance, scanability, Reliability Lab and perspective helpers.

The old prebuilt CommonJS distribution and immediate classic-script `quadqr.min.js` global are not the focus of this test port yet. Validate the browser/ESM WASM replacement first before replacing the published package distribution.

## Build artifacts

Generated `.wasm` and JavaScript glue are not committed. `run-wasm-demo.bat` / `run-wasm-demo.sh` performs setup, source checks, all WASM builds, generated-artifact checks, API smoke tests, and then starts the isolated local server.

If you need to separate a baseline WASM problem from a threads-specific problem, set `QUADQR_SKIP_THREADED=1` and run `npm run verify` first.


## Browser diagnostics

If the full demo does not start correctly, open `http://127.0.0.1:8080/demo/browser-smoke.html`. It tests baseline WASM import, encode, matrix decode, render and image scanning independently. Threaded Rayon is tested only when you press the dedicated button.
