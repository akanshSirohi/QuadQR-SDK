let activeApi = null;
let activeMode = "uninitialized";
let activeThreads = 0;
let singlePromise = null;
let threadedPromise = null;
let lastThreadError = null;

function isNodeRuntime() {
  return Boolean(globalThis.process?.versions?.node);
}

function singleModuleUrl() {
  return new URL(isNodeRuntime() ? "../wasm/node/quadqr_wasm.js" : "../wasm/single/quadqr_wasm.js", import.meta.url);
}

function threadedModuleUrl() {
  return new URL("../wasm/threaded/quadqr_wasm.js", import.meta.url);
}

function normalizeThreadPreference(value) {
  if (value === true || value === "threaded" || value === "force") return "force";
  if (value === false || value === "single" || value === "off") return "single";
  return "auto";
}

function supportsSharedWasmMemory() {
  if (typeof WebAssembly === "undefined" || typeof SharedArrayBuffer === "undefined") return false;
  try {
    const memory = new WebAssembly.Memory({ initial: 1, maximum: 1, shared: true });
    return memory.buffer instanceof SharedArrayBuffer;
  } catch {
    return false;
  }
}

export function supportsWasmThreads() {
  if (isNodeRuntime()) return false;
  return Boolean(
    globalThis.crossOriginIsolated &&
    typeof Worker !== "undefined" &&
    (globalThis.navigator?.hardwareConcurrency ?? 1) > 1 &&
    supportsSharedWasmMemory()
  );
}

function resolvedThreadCount(options = {}) {
  const hardware = Math.max(1, Number(globalThis.navigator?.hardwareConcurrency || 1));
  const requested = Number(options.threadCount ?? options.threadsCount ?? 0);
  if (Number.isFinite(requested) && requested > 0) {
    return Math.max(1, Math.min(hardware, Math.floor(requested)));
  }
  return hardware;
}

function withTimeout(promise, timeoutMs, label) {
  const ms = Math.max(250, Number(timeoutMs) || 0);
  if (!ms) return promise;
  let timer = null;
  const timeout = new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error(`${label} timed out after ${ms} ms.`)), ms);
  });
  return Promise.race([promise, timeout]).finally(() => {
    if (timer) clearTimeout(timer);
  });
}

function generatedWasmUrl(moduleJsUrl) {
  return new URL("./quadqr_wasm_bg.wasm", moduleJsUrl);
}

async function initializeGeneratedWebModule(imported, moduleJsUrl) {
  if (typeof imported.default !== "function") {
    throw new Error("QuadQR generated WASM module has no initialization function.");
  }
  // wasm-bindgen's current web target expects a single options object. Passing
  // the path explicitly also removes ambiguity around worker/import.meta.url
  // resolution and avoids the deprecated positional/no-argument init path.
  await imported.default({ module_or_path: generatedWasmUrl(moduleJsUrl) });
  return imported;
}

async function loadSingle() {
  if (!singlePromise) {
    singlePromise = (async () => {
      const moduleUrl = singleModuleUrl();
      const imported = await import(moduleUrl);
      if (isNodeRuntime()) {
        return imported.default && typeof imported.default.encodeText === "function"
          ? imported.default
          : imported;
      }
      return initializeGeneratedWebModule(imported, moduleUrl);
    })();
  }
  return singlePromise;
}

async function loadThreaded(options = {}) {
  if (!threadedPromise) {
    threadedPromise = (async () => {
      const moduleUrl = threadedModuleUrl();
      const imported = await import(moduleUrl);
      const module = await withTimeout(
        initializeGeneratedWebModule(imported, moduleUrl),
        options.wasmInitTimeoutMs ?? 8000,
        "Threaded QuadQR WASM initialization"
      );
      if (typeof module.initThreadPool !== "function") {
        throw new Error("Threaded QuadQR WASM build does not export initThreadPool(). Rebuild it with the threads feature.");
      }
      const threadCount = resolvedThreadCount(options);
      await withTimeout(
        module.initThreadPool(threadCount),
        options.threadInitTimeoutMs ?? 5000,
        "QuadQR Rayon thread-pool initialization"
      );
      return { module, threadCount };
    })();
  }
  return threadedPromise;
}

/**
 * Initialize or upgrade the QuadQR WASM runtime.
 *
 * Existing compatibility: calling initWasm() with no arguments remains valid.
 * New optional controls:
 *   initWasm({ threads: "auto" | true | false, threadCount: N })
 */
export async function initWasm(options = {}) {
  const preference = normalizeThreadPreference(options.threads);

  if (preference !== "single" && supportsWasmThreads()) {
    if (activeMode === "threaded" && activeApi) {
      return getWasmState();
    }
    try {
      const { module, threadCount } = await loadThreaded(options);
      activeApi = module;
      activeMode = "threaded";
      activeThreads = threadCount;
      lastThreadError = null;
      return getWasmState();
    } catch (error) {
      lastThreadError = error;
      if (preference === "force") throw error;
    }
  } else if (preference === "force") {
    throw new Error(
      "Threaded QuadQR WASM requires cross-origin isolation, SharedArrayBuffer, Web Workers, and more than one hardware thread."
    );
  }

  if (!activeApi || activeMode === "uninitialized") {
    activeApi = await loadSingle();
    activeMode = "single";
    activeThreads = 1;
  }
  return getWasmState();
}

export function getWasm() {
  if (!activeApi) {
    throw new Error("QuadQR WASM is not initialized yet.");
  }
  return activeApi;
}

export function getWasmState() {
  return Object.freeze({
    ready: Boolean(activeApi),
    backend: activeMode,
    threaded: activeMode === "threaded",
    threads: activeThreads,
    crossOriginIsolated: Boolean(globalThis.crossOriginIsolated),
    supportsThreads: supportsWasmThreads(),
    threadError: lastThreadError?.message ?? null
  });
}


/**
 * Legacy compatibility hook. The previous pure-JS package could disable its
 * optional accelerator. This package is WASM-backed by design, so disabling
 * acceleration means returning to the baseline single-thread WASM backend.
 * The switch is synchronous when that baseline instance has already been used;
 * otherwise callers should use initSingleThreadWasm().
 */
export function disableWasm() {
  lastThreadError = null;
  if (activeMode === "single") return getWasmState();
  // A synchronous public hook cannot await a first-time baseline import. Keep
  // the current backend alive rather than leaving the codec unusable.
  if (!singlePromise) return getWasmState();
  void singlePromise.then(module => {
    activeApi = module;
    activeMode = "single";
    activeThreads = 1;
  });
  return getWasmState();
}

/** Force the baseline build. Mostly useful for tests and compatibility checks. */
export async function initSingleThreadWasm() {
  activeApi = await loadSingle();
  activeMode = "single";
  activeThreads = 1;
  return getWasmState();
}

/** Explicitly request the true Rayon/WebAssembly threaded build. */
export async function initThreadedWasm(options = {}) {
  return initWasm({ ...options, threads: true });
}

// Preserve synchronous public codec functions by establishing the baseline
// WASM instance during module evaluation. Threaded WASM is deliberately NOT
// initialized here: workers must become responsive first, then opt into Rayon
// through initWasm(). This prevents a stalled browser thread-pool bootstrap
// from hanging the entire worker before its message handler exists.
await initWasm({ threads: false });
