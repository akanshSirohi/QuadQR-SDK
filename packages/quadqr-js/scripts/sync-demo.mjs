import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const demoDir = resolve(root, "demo");

// Keep the WASM-adapted demo HTML/app/worker files local. Only reference
// styling is synchronized so runtime fixes and the intentionally reduced tab
// set cannot be overwritten by the JavaScript reference demo.
const reference = process.env.QUADQR_JS_REFERENCE ?? "0fdbab50c0ed4329017e201ef535c99730dc58ec";
const base = `https://raw.githubusercontent.com/akanshSirohi/QuadQR/${reference}/demo`;

async function fetchText(name) {
  const response = await fetch(`${base}/${name}`, { redirect: "follow" });
  if (!response.ok) {
    throw new Error(`Unable to fetch ${name} from QuadQR JS reference ${reference}: HTTP ${response.status}`);
  }
  return response.text();
}

await mkdir(demoDir, { recursive: true });
const styles = await fetchText("styles.css");
await writeFile(resolve(demoDir, "styles.css"), styles, "utf8");
console.log(`synced demo/styles.css from QuadQR JS reference ${reference}`);
console.log("Rust/WASM demo HTML, app and worker files were intentionally preserved.");
