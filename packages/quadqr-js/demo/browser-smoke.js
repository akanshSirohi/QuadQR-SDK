const rows = document.querySelector('#rows');
const log = document.querySelector('#log');
const canvas = document.querySelector('#preview');
const threadBtn = document.querySelector('#threadBtn');
const tests = new Map();

function row(name) {
  if (tests.has(name)) return tests.get(name);
  const tr = document.createElement('tr');
  tr.innerHTML = `<td>${name}</td><td class="run">pending</td><td></td>`;
  rows.appendChild(tr);
  const item = { tr, status: tr.children[1], details: tr.children[2] };
  tests.set(name, item);
  return item;
}
function set(name, state, details = '') {
  const item = row(name);
  item.status.className = state === 'PASS' ? 'ok' : state === 'FAIL' ? 'bad' : 'run';
  item.status.textContent = state;
  item.details.textContent = details;
}
function append(value) {
  log.textContent += `${value}\n`;
}
async function timed(name, fn) {
  set(name, 'RUNNING');
  const started = performance.now();
  try {
    const result = await fn();
    set(name, 'PASS', `${(performance.now() - started).toFixed(1)} ms`);
    return result;
  } catch (error) {
    set(name, 'FAIL', error?.message ?? String(error));
    append(`${name}: ${error?.stack ?? error}`);
    throw error;
  }
}

let api = null;
let code = null;
let rendered = null;
try {
  api = await timed('Import JS compatibility layer', () => import('../library/quadqr.js'));
  await timed('Baseline WASM ready', async () => {
    const state = await api.initWasm({ threads: false });
    append(`baseline state: ${JSON.stringify(state)}`);
    if (!state.ready || state.backend !== 'single') throw new Error(`Unexpected state: ${JSON.stringify(state)}`);
  });
  code = await timed('Rust encodeText', () => api.encodeText('QuadQR browser smoke test', { ecc: 'M', compression: 'none' }));
  await timed('Rust decodeMatrix', () => {
    const decoded = api.decodeMatrix(code.matrix);
    if (decoded.text !== 'QuadQR browser smoke test') throw new Error(`Roundtrip mismatch: ${decoded.text}`);
    return decoded;
  });
  rendered = await timed('Render ImageData', () => api.renderToImageData(code, { imageSize: 320, quietZone: 4, style: 'classic' }));
  await timed('Draw preview', () => {
    const ctx = canvas.getContext('2d');
    ctx.putImageData(rendered instanceof ImageData ? rendered : new ImageData(rendered.data, rendered.width, rendered.height), 0, 0);
  });
  await timed('Rust image scanner', () => {
    const result = api.scanImageData(rendered, {
      minVersion: code.version,
      maxVersion: code.version,
      finderRecovery: false,
      rotationRecovery: false
    });
    if (result.text !== 'QuadQR browser smoke test') throw new Error(`Image scan mismatch: ${result.text}`);
    return result;
  });
  append('Baseline browser pipeline passed.');
} catch (error) {
  append(`Baseline pipeline stopped: ${error?.message ?? error}`);
}

threadBtn.addEventListener('click', async () => {
  threadBtn.disabled = true;
  try {
    if (!api) api = await import('../library/quadqr.js');
    await timed('Thread support', async () => {
      const wasm = await import('../library/wasm.js');
      if (!wasm.supportsWasmThreads()) throw new Error('Browser is not cross-origin isolated or shared WASM memory is unavailable.');
    });
    await timed('Rayon thread pool', async () => {
      const state = await Promise.race([
        api.initWasm({ threads: true, threadCount: Math.min(4, navigator.hardwareConcurrency || 2) }),
        new Promise((_, reject) => setTimeout(() => reject(new Error('Threaded initialization exceeded 8 seconds.')), 8000))
      ]);
      append(`threaded state: ${JSON.stringify(state)}`);
      if (!state.threaded) throw new Error(`Threaded backend not active: ${JSON.stringify(state)}`);
    });
  } catch (error) {
    append(`Threaded test stopped: ${error?.stack ?? error}`);
  } finally {
    threadBtn.disabled = false;
  }
});
