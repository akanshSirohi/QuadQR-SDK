"""Render the maintained Markdown API guides as static documentation pages."""

from html import escape
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "documentation"
sys.path.insert(0, str(ROOT / ".tools" / "doc-deps"))

try:
    import markdown
except ImportError as error:
    raise SystemExit(
        "Install documentation dependencies with: "
        "python -m pip install --target .tools/doc-deps -r scripts/requirements-docs.txt"
    ) from error

GUIDES = [
    ("java-guide.html", ROOT / "android/JAVA.md", "Java API", "The complete Java guide for the Android AAR."),
    ("kotlin-guide.html", ROOT / "android/KOTLIN.md", "Kotlin API", "The complete Kotlin guide for the Android AAR."),
    ("dart-guide.html", ROOT / "packages/quadqr-flutter/README.md", "Dart API", "The complete Flutter Android SDK guide."),
    ("wasm-guide.html", ROOT / "packages/quadqr-js/README.md", "WASM package", "Build and use the experimental Rust-backed JavaScript package."),
    ("contributing.html", ROOT / "CONTRIBUTING.md", "Contributing", "Development checks, compatibility rules, and pull request guidance."),
]

template = (DOCS / "rust.html").read_text(encoding="utf-8")
article_pattern = re.compile(
    r'(<article class="article">).*?(<footer class="footer">)', re.DOTALL
)

for output_name, source, title, lead in GUIDES:
    source_text = source.read_text(encoding="utf-8")
    source_text = re.sub(r"\A# [^\n]+\n+", "", source_text, count=1)
    content = markdown.markdown(source_text, extensions=["extra", "toc"])
    if output_name == "contributing.html":
        content = content.replace('href="README.md#requirements"', 'href="build.html"')
    header = (
        f'<header><span class="eyebrow">Source guide</span>'
        f'<h1>{escape(title)}</h1><p class="lead">{escape(lead)}</p></header>'
    )
    page = re.sub(
        r"<title>[^<]+</title>",
        f"<title>{escape(title)} | QuadQR documentation</title>",
        template,
        count=1,
    )
    page, replacements = article_pattern.subn(
        lambda match: match.group(1) + header + content + match.group(2),
        page,
        count=1,
    )
    if replacements != 1:
        raise SystemExit("Could not find documentation article template")
    (DOCS / output_name).write_text(page, encoding="utf-8")
    print(f"Rendered {output_name} from {source.relative_to(ROOT)}")

license_text = (ROOT / "LICENSE").read_text(encoding="utf-8")
license_page = re.sub(
    r"<title>[^<]+</title>",
    "<title>License | QuadQR documentation</title>",
    template,
    count=1,
)
license_header = (
    '<header><span class="eyebrow">Project</span><h1>License</h1>'
    '<p class="lead">QuadQR SDK is licensed under AGPL-3.0-only.</p></header>'
)
license_page, replacements = article_pattern.subn(
    lambda match: match.group(1) + license_header
    + '<pre class="license-text"><code>' + escape(license_text) + '</code></pre>'
    + match.group(2),
    license_page,
    count=1,
)
if replacements != 1:
    raise SystemExit("Could not find documentation article template")
(DOCS / "license.html").write_text(license_page, encoding="utf-8")
print("Rendered license.html from LICENSE")
