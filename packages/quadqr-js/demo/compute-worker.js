const params = new URL(import.meta.url).searchParams;
const threaded = params.get("threads") === "1";

// Always let the module boot its baseline backend first. If this worker is used
// for analysis, upgrade to Rayon only after module evaluation has completed.
// A stalled threaded bootstrap therefore cannot prevent the worker from ever
// installing its message handler.
const quadqr = await import("../library/quadqr.js");
const benchmarkApi = await import("../library/benchmark.js");
const wasmRuntime = await import("../library/wasm.js");

const {
  encodeText,
  encodeSignedText,
  encodeSecureText,
  scanImageData,
  runImageStressTest,
  runReliabilityLab,
  runPerspectiveSweep,
  applyStressDistortion
} = quadqr;
const { benchmarkCodec } = benchmarkApi;
const scannerWasmReady = threaded
  ? wasmRuntime.initWasm({ threads: "auto", wasmInitTimeoutMs: 8000, threadInitTimeoutMs: 5000 })
      .catch(() => wasmRuntime.initWasm({ threads: false }))
  : Promise.resolve(wasmRuntime.getWasmState());

self.postMessage({ type: "worker-ready", wasm: wasmRuntime.getWasmState(), threadedRequested: threaded });

function serializeError(error) {
  return {
    name: error?.name || "Error",
    message: error?.message || String(error),
    stack: error?.stack || "",
    debug: error?.debug ?? null
  };
}

function imageDataFromPayload(value) {
  if (!value?.data || !value.width || !value.height) {
    throw new Error("Worker expected ImageData-compatible input.");
  }
  const data = value.data instanceof Uint8ClampedArray
    ? value.data
    : new Uint8ClampedArray(value.data);
  return { data, width: value.width, height: value.height };
}

async function handleTask(task, payload) {
  if (task === "encode") {
    const { text, commonOptions, compression, security, signing } = payload;
    if (security) {
      return encodeSecureText(text, {
        ...commonOptions,
        compression,
        security,
        ...(signing ? { signing } : {})
      });
    }
    if (signing) {
      return encodeSignedText(text, {
        ...commonOptions,
        compression,
        ...signing
      });
    }
    return encodeText(text, { ...commonOptions, compression });
  }

  if (task === "scan") {
    await scannerWasmReady;
    return scanImageData(imageDataFromPayload(payload.imageData), payload.options ?? {});
  }

  if (task === "stress") {
    await scannerWasmReady;
    return runImageStressTest(imageDataFromPayload(payload.imageData), payload.expected ?? {});
  }

  if (task === "reliability") {
    await scannerWasmReady;
    return runReliabilityLab(
      imageDataFromPayload(payload.imageData),
      payload.expected ?? {},
      payload.options ?? {}
    );
  }

  if (task === "perspective-test") {
    await scannerWasmReady;
    const source = imageDataFromPayload(payload.imageData);
    const distorted = applyStressDistortion(
      source,
      "perspective-3d",
      0.5,
      payload.transform ?? {}
    );
    let decoded = null;
    let error = null;
    try {
      decoded = scanImageData(distorted, payload.scanOptions ?? {});
    } catch (scanError) {
      error = serializeError(scanError);
    }
    return {
      decoded,
      error,
      distorted: {
        width: distorted.width,
        height: distorted.height,
        data: distorted.data
      }
    };
  }

  if (task === "perspective-sweep") {
    await scannerWasmReady;
    return runPerspectiveSweep(
      imageDataFromPayload(payload.imageData),
      payload.expected ?? {},
      payload.options ?? {}
    );
  }

  if (task === "benchmark") {
    const normal = benchmarkCodec({ ...(payload.options ?? {}), highDensity: false });
    const highDensity = benchmarkCodec({ ...(payload.options ?? {}), highDensity: true });
    return { normal, highDensity };
  }

  throw new Error(`Unknown worker task: ${task}`);
}

self.addEventListener("message", async (event) => {
  const { id, task, payload } = event.data ?? {};
  if (!id || !task) return;
  try {
    const result = await handleTask(task, payload ?? {});
    self.postMessage({ id, ok: true, result });
  } catch (error) {
    self.postMessage({ id, ok: false, error: serializeError(error) });
  }
});
