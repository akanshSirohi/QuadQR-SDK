// Camera workers load the baseline WASM during module startup. The page waits
// for this worker's explicit boot message before sending init, which avoids a
// module-worker race where init can arrive while imports are still resolving.
// Live camera scanning defaults to baseline single-thread WASM; threaded mode
// remains an explicit opt-in for callers that want to test it.
const wasmRuntime = await import("./wasm.js");
const quadqr = await import("./quadqr.js");
const { initWasm, getWasmState } = wasmRuntime;
const { scanImageData } = quadqr;

let ready = false;
let initializing = false;
let initPromise = Promise.resolve(getWasmState());
let missStreak = 0;
let initFailure = null;

function serializeError(error) {
  return { name: error?.name || "Error", message: error?.message || String(error), stack: error?.stack || "" };
}

self.addEventListener("message", async event => {
  const message = event.data ?? {};
  if (message.type === "init") {
    if (initializing || ready) {
      if (ready) self.postMessage({ type: "ready", ...getWasmState() });
      return;
    }
    initializing = true;
    ready = false;
    initFailure = null;
    self.postMessage({ type: "init-start", threads: message.threads ?? false });
    initPromise = initWasm({
      threads: message.threads ?? false,
      threadCount: message.threadCount ?? undefined,
      wasmInitTimeoutMs: message.wasmInitTimeoutMs ?? 8000,
      threadInitTimeoutMs: message.threadInitTimeoutMs ?? 5000
    })
      .then(state => {
        initializing = false;
        ready = true;
        self.postMessage({ type: "ready", ...state });
        return state;
      })
      .catch(error => {
        initializing = false;
        ready = false;
        initFailure = error;
        self.postMessage({ type: "init-error", error: serializeError(error) });
        return null;
      });
    return;
  }
  if (message.type !== "scan") return;
  if (!ready) await initPromise;
  if (!ready) {
    self.postMessage({
      type: "scan-result",
      ok: false,
      frame: message.frame,
      error: serializeError(initFailure ?? new Error("QuadQR scanner worker is not initialized."))
    });
    return;
  }
  const started = performance.now();
  const requested = message.options ?? {};
  const recoveryAfter = Math.max(0, Math.floor(Number(requested.cameraFinderRecoveryAfter ?? 3)));
  const recoveryEvery = Math.max(1, Math.floor(Number(requested.cameraFinderRecoveryEvery ?? 4)));
  const useDeepRecovery = requested.finderRecovery !== false &&
    missStreak >= recoveryAfter &&
    ((missStreak - recoveryAfter) % recoveryEvery === 0);
  try {
    const rgba = message.rgba instanceof Uint8ClampedArray
      ? message.rgba
      : new Uint8ClampedArray(message.rgba);
    const imageData = { width: message.width, height: message.height, data: rgba };
    const scanOptions = useDeepRecovery
      ? requested
      : { ...requested, finderRecovery: false, rotationRecovery: false };
    const result = scanImageData(imageData, scanOptions);
    missStreak = 0;
    self.postMessage({
      type: "scan-result", ok: true, frame: message.frame,
      result, width: message.width, height: message.height, rgba,
      deepRecovery: useDeepRecovery,
      backend: getWasmState()
    }, [rgba.buffer]);
  } catch (error) {
    missStreak++;
    const elapsedMs = performance.now() - started;
    self.postMessage({
      type: "scan-result", ok: false, frame: message.frame,
      error: serializeError(error),
      diagnostic: {
        type: "frame",
        state: "searching",
        frame: message.frame,
        elapsedMs,
        method: getWasmState().threaded
          ? (useDeepRecovery ? "rust-wasm-parallel-recovery" : "rust-wasm-parallel-fast")
          : (useDeepRecovery ? "rust-wasm-recovery" : "rust-wasm-fast"),
        finderCount: 0,
        missStreak,
        deepRecovery: useDeepRecovery,
        scanWidth: message.width,
        scanHeight: message.height,
        frameWidth: message.width,
        frameHeight: message.height,
        finderMethod: "rust-finder"
      }
    });
  }
});

// Boot is announced only after the message listener above is installed. The
// page responds to this message with the init command, so init can never be
// lost while this module is still loading its WASM dependencies.
self.postMessage({ type: "boot", ...getWasmState() });
