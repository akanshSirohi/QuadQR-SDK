"""Check local links in the README and static documentation site."""

from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit
import re

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "documentation"
PAGES = [ROOT / "README.md", *sorted(DOCS.glob("*.html"))]
missing = []


def check(source: Path, target: str) -> None:
    if not target or target.startswith(("#", "//")):
        return
    parts = urlsplit(target)
    if parts.scheme:
        return
    path = (source.parent / unquote(parts.path)).resolve()
    if source.suffix == ".html" and (path.suffix == ".md" or not path.is_relative_to(DOCS)):
        missing.append((source.relative_to(ROOT), f"outside site: {target}"))
        return
    if not path.exists():
        missing.append((source.relative_to(ROOT), target))


class Links(HTMLParser):
    def __init__(self, source: Path) -> None:
        super().__init__()
        self.source = source

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        for key, value in attrs:
            if key in {"href", "src"} and value:
                check(self.source, value)


for source in PAGES:
    content = source.read_text(encoding="utf-8")
    if source.suffix == ".html":
        Links(source).feed(content)
    else:
        for target in re.findall(r"!?(?:\[[^]]*\])\(([^)]+)\)", content):
            check(source, target)

if missing:
    for source, target in missing:
        print(f"{source}: missing {target}")
    raise SystemExit(1)

print(f"Checked local links in {len(PAGES)} documentation files.")
