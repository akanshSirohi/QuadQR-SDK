import http from "node:http";
import { createReadStream, existsSync, statSync } from "node:fs";
import { extname, join, normalize, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const port = Number(process.env.PORT ?? 8080);

const contentTypes = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".wasm": "application/wasm",
  ".json": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".webp": "image/webp"
};

function safePath(urlPath) {
  const clean = decodeURIComponent(urlPath.split("?")[0]);
  const requested = clean === "/" ? "/demo/index.html" : clean;
  const absolute = resolve(root, `.${normalize(requested)}`);
  return absolute.startsWith(root) ? absolute : null;
}

const server = http.createServer((req, res) => {
  let path = safePath(req.url ?? "/");
  if (!path) {
    res.writeHead(403).end("Forbidden");
    return;
  }
  if (existsSync(path) && statSync(path).isDirectory()) path = join(path, "index.html");
  if (!existsSync(path) || !statSync(path).isFile()) {
    res.writeHead(404, { "Content-Type": "text/plain; charset=utf-8" }).end("Not found");
    return;
  }

  res.setHeader("Cross-Origin-Opener-Policy", "same-origin");
  res.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
  res.setHeader("Cross-Origin-Resource-Policy", "same-origin");
  res.setHeader("Origin-Agent-Cluster", "?1");
  res.setHeader("Cache-Control", "no-cache");
  res.setHeader("Content-Type", contentTypes[extname(path).toLowerCase()] ?? "application/octet-stream");
  createReadStream(path).pipe(res);
});

server.listen(port, "127.0.0.1", () => {
  console.log(`QuadQR WASM demo: http://127.0.0.1:${port}/demo/`);
  console.log("COOP/COEP headers are enabled so threaded WASM can use SharedArrayBuffer.");
});
