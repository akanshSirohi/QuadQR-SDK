/* QuadQR static-host cross-origin isolation helper.
 * Local/Dokploy servers should prefer real COOP/COEP response headers. On a
 * static host such as GitHub Pages this service worker can add them to the
 * controlled navigation after one automatic reload, enabling WASM threads.
 */
if (typeof window === "undefined") {
  self.addEventListener("install", () => self.skipWaiting());
  self.addEventListener("activate", event => event.waitUntil(self.clients.claim()));
  self.addEventListener("fetch", event => {
    const request = event.request;
    if (request.cache === "only-if-cached" && request.mode !== "same-origin") return;
    event.respondWith((async () => {
      const response = await fetch(request);
      if (response.type === "opaque" || response.status === 0) return response;
      const headers = new Headers(response.headers);
      headers.set("Cross-Origin-Opener-Policy", "same-origin");
      headers.set("Cross-Origin-Embedder-Policy", "require-corp");
      if (new URL(request.url).origin === self.location.origin) {
        headers.set("Cross-Origin-Resource-Policy", "same-origin");
      }
      return new Response(response.body, {
        status: response.status,
        statusText: response.statusText,
        headers
      });
    })());
  });
} else if ("serviceWorker" in navigator && window.isSecureContext && !window.crossOriginIsolated) {
  const key = "quadqr-coi-reloaded";
  navigator.serviceWorker.register("./coi-serviceworker.js").then(async registration => {
    await navigator.serviceWorker.ready;
    if (!navigator.serviceWorker.controller && sessionStorage.getItem(key) !== "1") {
      sessionStorage.setItem(key, "1");
      location.reload();
      return;
    }
    if (navigator.serviceWorker.controller && sessionStorage.getItem(key) !== "1") {
      sessionStorage.setItem(key, "1");
      location.reload();
    }
  }).catch(() => {});
} else if (window.crossOriginIsolated) {
  try { sessionStorage.removeItem("quadqr-coi-reloaded"); } catch {}
}
