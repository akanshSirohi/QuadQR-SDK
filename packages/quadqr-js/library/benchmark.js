/** Compatibility benchmark helpers backed by the Rust/WASM QuadQR API. */
import {
  encodeBytes,
  decodeMatrix,
  getVersionInfo,
  compressPayload,
  compressDeflatePayload,
  compressBrotliPayload,
  MAX_VERSION
} from "./quadqr.js";

export const STANDARD_QR_BYTE_CAPACITY = Object.freeze({
  L: Object.freeze([17,32,53,78,106,134,154,192,230,271,321,367,425,458,520,586,644,718,792,858,929,1003,1091,1171,1273,1367,1465,1528,1628,1732,1840,1952,2068,2188,2303,2431,2563,2699,2809,2953]),
  M: Object.freeze([14,26,42,62,84,106,122,152,180,213,251,287,331,362,412,450,504,560,624,666,711,779,857,911,997,1059,1125,1190,1264,1370,1452,1538,1628,1722,1809,1911,1989,2099,2213,2331]),
  Q: Object.freeze([11,20,32,46,60,74,86,108,130,151,177,203,241,258,292,322,364,394,442,482,509,565,611,661,715,751,805,868,908,982,1030,1112,1168,1228,1283,1351,1423,1499,1579,1663]),
  H: Object.freeze([7,14,24,34,44,58,64,84,98,119,137,155,177,194,220,250,280,310,338,382,403,439,461,511,535,593,625,658,698,742,790,842,898,958,983,1051,1093,1139,1219,1273])
});

function normalizeEcc(ecc = "M") {
  const value = String(ecc).toUpperCase();
  if (!STANDARD_QR_BYTE_CAPACITY[value]) throw new Error("ECC must be one of L, M, Q, H.");
  return value;
}
function resolveHighDensity(options = {}) {
  return typeof options.highDensity === "boolean" ? options.highDensity : options.cellEncoding === "triangle16";
}
function nowMs() { return globalThis.performance?.now?.() ?? Date.now(); }
function makePayload(length, seed = 0x51) {
  const out = new Uint8Array(length); let state = (seed ^ length) >>> 0;
  for (let i=0;i<length;i++) { state ^= state << 13; state ^= state >>> 17; state ^= state << 5; out[i] = state & 0xff; }
  return out;
}
function summarize(samples) {
  if (!samples.length) return { meanMs:0, medianMs:0, p95Ms:0, minMs:0, maxMs:0 };
  const sorted = samples.slice().sort((a,b)=>a-b);
  const pick = p => sorted[Math.min(sorted.length-1, Math.max(0, Math.ceil(p*sorted.length)-1))];
  return { meanMs: samples.reduce((a,b)=>a+b,0)/samples.length, medianMs:pick(.5), p95Ms:pick(.95), minMs:sorted[0], maxMs:sorted.at(-1) };
}

export function getStandardQrByteCapacity(version, ecc = "M") {
  if (!Number.isInteger(version) || version < 1 || version > MAX_VERSION) throw new Error(`Version must be 1..${MAX_VERSION}.`);
  return STANDARD_QR_BYTE_CAPACITY[normalizeEcc(ecc)][version-1];
}

export function compareCapacity(version, ecc = "M", options = {}) {
  const level = normalizeEcc(ecc); const highDensity = resolveHighDensity(options);
  const quadqr = getVersionInfo(version, { ecc: level, highDensity });
  const standardQrBytes = getStandardQrByteCapacity(version, level);
  const quadqrBytes = quadqr.capacityBytes; const differenceBytes = quadqrBytes-standardQrBytes;
  return {
    version, size:quadqr.size, ecc:level, quadqrBytes, standardQrBytes, differenceBytes,
    ratio: standardQrBytes ? quadqrBytes/standardQrBytes : null,
    gainPercent: standardQrBytes ? differenceBytes/standardQrBytes*100 : null,
    highDensity, cellEncoding:quadqr.cellEncoding, quadqrBitsPerDataCell:quadqr.bitsPerDataCell,
    quadqrPayloadEfficiencyPercent: quadqr.theoreticalBits ? quadqrBytes*8/quadqr.theoreticalBits*100 : 0,
    quadqrPayloadBitsPerMatrixCell: quadqrBytes*8/(quadqr.size*quadqr.size),
    standardQrPayloadBitsPerMatrixCell: standardQrBytes*8/(quadqr.size*quadqr.size),
    note:"ECC profile letters are nominal only; recovery strength is not equivalent to ISO QR."
  };
}

export function buildCapacityComparison(options = {}) {
  const ecc = normalizeEcc(options.ecc ?? "M"); const highDensity = resolveHighDensity(options);
  const versions = options.versions ?? Array.from({length:MAX_VERSION},(_,i)=>i+1);
  return versions.map(version=>compareCapacity(version,ecc,{highDensity}));
}

export function calculateCapacityPlan(options = {}) {
  const ecc = normalizeEcc(options.ecc ?? "M"); const highDensity = resolveHighDensity(options);
  let sourceBytes;
  const concrete = options.payload instanceof Uint8Array || typeof options.payload === "string";
  if (options.payload instanceof Uint8Array) sourceBytes = options.payload;
  else if (typeof options.payload === "string") sourceBytes = new TextEncoder().encode(options.payload);
  else sourceBytes = new Uint8Array(Math.max(0, Math.floor(options.payloadBytes ?? 0)));
  const requested = String(options.compression ?? "none").toLowerCase();
  if (!["none","auto","smart","lz","deflate","brotli"].includes(requested)) throw new Error("compression must be none, auto, smart, lz, deflate, or brotli.");
  let compression = requested, compressionLevel = null, storedBytes = sourceBytes.length, compressed = false;
  if (requested !== "none" && concrete) {
    const candidates=[]; const explicit=options.compressionLevel;
    if (["auto","smart","lz"].includes(requested)) { const level=requested==="lz"?(explicit??options.lzLevel??6):6; candidates.push({compression:"lz",bytes:compressPayload(sourceBytes,{level}).length,level}); }
    if (requested === "auto") { candidates.push({compression:"deflate",bytes:compressDeflatePayload(sourceBytes,{level:6}).length,level:6}); candidates.push({compression:"brotli",bytes:compressBrotliPayload(sourceBytes,{quality:6}).length,level:6}); }
    else if (requested === "smart") { for (const level of [6,8,9]) candidates.push({compression:"deflate",bytes:compressDeflatePayload(sourceBytes,{level}).length,level}); for (const level of [6,9,11]) candidates.push({compression:"brotli",bytes:compressBrotliPayload(sourceBytes,{quality:level}).length,level}); }
    else if (requested === "deflate") { const level=explicit??options.deflateLevel??6; candidates.push({compression:"deflate",bytes:compressDeflatePayload(sourceBytes,{level}).length,level}); }
    else if (requested === "brotli") { const level=explicit??options.brotliQuality??11; candidates.push({compression:"brotli",bytes:compressBrotliPayload(sourceBytes,{quality:level}).length,level}); }
    candidates.sort((a,b)=>a.bytes-b.bytes); const best=candidates[0];
    if (best && (!["auto","smart"].includes(requested) || best.bytes+16 < sourceBytes.length)) { storedBytes=best.bytes; compressed=true; compression=best.compression; compressionLevel=best.level; } else { compression="none"; }
  } else if (requested !== "none" && !concrete) compression="unknown";
  const signed=Boolean(options.signed); const keyIdBytes=options.keyId?new TextEncoder().encode(String(options.keyId)).length:0;
  const signingBytes=signed?64+keyIdBytes+(options.embedPublicKey?32:0):0;
  const needsEnvelope=signed||compressed||["lz","deflate","brotli"].includes(requested);
  const encodedBytes=storedBytes+(needsEnvelope?16+signingBytes:0); const extensionOverheadBytes=encodedBytes-storedBytes;
  let quadqrVersion=null, quadqrInfo=null;
  for(let version=1;version<=MAX_VERSION;version++){const info=getVersionInfo(version,{ecc,highDensity});if(encodedBytes<=info.capacityBytes){quadqrVersion=version;quadqrInfo=info;break;}}
  let standardQrVersion=null; for(let version=1;version<=MAX_VERSION;version++){if(sourceBytes.length<=STANDARD_QR_BYTE_CAPACITY[ecc][version-1]){standardQrVersion=version;break;}}
  const standardQrSize=standardQrVersion?21+4*(standardQrVersion-1):null;
  return { ecc,highDensity,cellEncoding:highDensity?"triangle16":"rgbw",sourceBytes:sourceBytes.length,encodedBytes,storedBytes,extensionOverheadBytes,compression,compressionLevel,compressed,signed,
    quadqrVersion,quadqrSize:quadqrInfo?.size??null,quadqrCapacityBytes:quadqrInfo?.capacityBytes??null,remainingBytes:quadqrInfo?quadqrInfo.capacityBytes-encodedBytes:null,
    utilizationPercent:quadqrInfo?encodedBytes/quadqrInfo.capacityBytes*100:null,standardQrVersion,standardQrSize,
    matrixWidthSavingsPercent:quadqrInfo&&standardQrSize?(1-quadqrInfo.size/standardQrSize)*100:null,
    note:"Compression is measured only when concrete payload bytes are provided. Byte-count-only estimates cannot predict compression gain." };
}

export function benchmarkCodec(options = {}) {
  const ecc=normalizeEcc(options.ecc??"M"); const highDensity=resolveHighDensity(options); const iterations=Math.max(1,Math.floor(options.iterations??30)); const warmup=Math.max(0,Math.floor(options.warmup??Math.min(5,iterations))); const requestedSizes=options.payloadSizes??[24,32,128,512,1024,2048]; const results=[];
  for(const requestedSize of requestedSizes){const payloadBytes=Math.max(0,Math.floor(requestedSize));const payload=makePayload(payloadBytes);let probe;try{probe=encodeBytes(payload,{ecc,highDensity});}catch(error){results.push({payloadBytes,skipped:true,reason:error.message});continue;}
    for(let i=0;i<warmup;i++){const encoded=encodeBytes(payload,{ecc,highDensity,version:probe.version});decodeMatrix(encoded.matrix);}
    const encodeSamples=[],decodeSamples=[];let encoded=probe;for(let i=0;i<iterations;i++){let start=nowMs();encoded=encodeBytes(payload,{ecc,highDensity,version:probe.version});encodeSamples.push(nowMs()-start);start=nowMs();const decoded=decodeMatrix(encoded.matrix);decodeSamples.push(nowMs()-start);if(decoded.payload.length!==payload.length)throw new Error(`Benchmark decode length mismatch at ${payloadBytes} bytes.`);}
    const info=getVersionInfo(encoded.version,{ecc,highDensity});results.push({payloadBytes,skipped:false,version:encoded.version,size:encoded.size,capacityBytes:info.capacityBytes,utilizationPercent:info.capacityBytes?payloadBytes/info.capacityBytes*100:0,encode:summarize(encodeSamples),decode:summarize(decodeSamples),iterations});}
  return {format:"QuadQR",ecc,highDensity,cellEncoding:highDensity?"triangle16":"rgbw",iterations,warmup,generatedAt:new Date().toISOString(),results};
}

export function benchmarkReport(options = {}) {
  const ecc=normalizeEcc(options.ecc??"M"); const highDensity=resolveHighDensity(options); const versions=options.versions??[1,2,5,10,20,30,40];
  return { capacity:buildCapacityComparison({ecc,versions,highDensity}), performance:benchmarkCodec({ecc,highDensity,iterations:options.iterations??30,warmup:options.warmup,payloadSizes:options.payloadSizes}) };
}
