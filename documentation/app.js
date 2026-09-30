(() => {
  const pages = [
    ["index.html", "Overview"],
    ["rust.html", "Rust"],
    ["android.html", "Android"],
    ["flutter.html", "Flutter"],
    ["web.html", "Web / WASM"],
    ["build.html", "Build and test"]
  ];
  const current = location.pathname.split("/").pop() || "index.html";
  const navigation = document.querySelector("#site-nav");
  if (navigation) {
    navigation.innerHTML = '<div class="nav-label">Documentation</div>' +
      pages.map(([href, label]) => `<a href="${href}"${href === current ? ' class="active" aria-current="page"' : ""}>${label}</a>`).join("") +
      '<div class="nav-label">Source guides</div>' +
      ['java-guide.html', 'kotlin-guide.html', 'dart-guide.html', 'wasm-guide.html']
        .map((href, index) => {
          const label = ['Java API', 'Kotlin API', 'Dart API', 'WASM package'][index];
          return `<a href="${href}"${href === current ? ' class="active" aria-current="page"' : ""}>${label}</a>`;
        }).join("") +
      '<div class="nav-label">Project</div>' +
      '<a href="contributing.html"' + (current === 'contributing.html' ? ' class="active" aria-current="page"' : '') + '>Contributing</a>' +
      '<a href="license.html"' + (current === 'license.html' ? ' class="active" aria-current="page"' : '') + '>License</a>' +
      '<div class="sidebar-note"><a href="https://github.com/akanshSirohi/QuadQR-SDK">Source repository</a><br>QuadQR SDK · AGPL-3.0-only</div>';
    const currentLink = navigation.querySelector('a[aria-current="page"]');
    if (currentLink) {
      const sections = document.createElement("div");
      sections.className = "nav-subpoints";
      const used = new Set();
      document.querySelectorAll(".article h2, .article h3").forEach((heading) => {
        if (!heading.id) {
          const base = heading.textContent.trim().toLowerCase()
            .replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "section";
          let id = base;
          for (let number = 2; used.has(id) || document.getElementById(id); number++) {
            id = `${base}-${number}`;
          }
          heading.id = id;
        }
        used.add(heading.id);
        const link = document.createElement("a");
        link.href = `#${heading.id}`;
        link.textContent = heading.textContent.replace(/¶/g, "").trim();
        if (heading.tagName === "H3") link.className = "level-3";
        sections.append(link);
      });
      currentLink.after(sections);
    }
  }
  document.querySelector("#menu-button")?.addEventListener("click", () => navigation?.classList.toggle("open"));
  document.querySelectorAll("#site-nav a").forEach(a => a.addEventListener("click", () => navigation?.classList.remove("open")));
  const theme = localStorage.getItem("quadqr-doc-theme");
  if (theme === "dark") document.documentElement.dataset.theme = "dark";
  document.querySelector("#theme-button")?.addEventListener("click", () => {
    const next = document.documentElement.dataset.theme === "dark" ? "light" : "dark";
    document.documentElement.dataset.theme = next;
    localStorage.setItem("quadqr-doc-theme", next);
  });
  document.querySelectorAll('pre code[class*="language-"]').forEach(code => {
    if (window.hljs) window.hljs.highlightElement(code);
  });
  document.querySelectorAll("pre").forEach(pre => {
    const button = document.createElement("button");
    button.className = "icon-button";
    button.type = "button";
    button.textContent = "⧉";
    button.setAttribute("aria-label", "Copy code");
    button.style.cssText = "float:right;margin:-.4rem -.55rem .2rem .4rem";
    button.addEventListener("click", async () => {
      await navigator.clipboard.writeText(pre.querySelector("code")?.textContent || pre.textContent);
      button.textContent = "✓";
      setTimeout(() => button.textContent = "⧉", 1400);
    });
    pre.prepend(button);
  });
})();
