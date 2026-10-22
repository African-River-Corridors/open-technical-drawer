"""Check the links on the docs page (website/index.html). Standard library only.

    python3 website/check_links.py

The page is served at https://www.africanrivercorridors.com/otd: the ARC site proxies /otd to this
repo's GitHub Pages site. So every local link is an absolute path, with no trailing slash and no .html:
- /otd/<file> must be a file in website/;
- any other absolute path must be an ARC site route listed in SITE_ROUTES;
- a relative link is an error (it resolves against the wrong folder behind the proxy).
External http(s) and mailto links are not fetched.
"""

import re
import sys
from pathlib import Path

SLUG = "otd"
HERE = Path(__file__).resolve().parent
SITE_ROUTES = {"/", "/opensource", "/ovcd", "/owd", "/otd", "/ord", "/dfs"}

errors = []
for page in sorted(HERE.glob("*.html")):
    for n, line in enumerate(page.read_text().splitlines(), 1):
        for url in re.findall(r'(?:href|src)="([^"]*)"', line):
            where = f"{page.name}:{n}: {url}"
            if url.startswith(("http://", "https://", "mailto:", "#")):
                continue
            if not url.startswith("/"):
                errors.append(f"{where}: relative link")
            elif url.startswith(f"/{SLUG}/"):
                if not (HERE / url[len(SLUG) + 2 :]).is_file():
                    errors.append(f"{where}: no such file in website/")
            elif url not in SITE_ROUTES:
                errors.append(f"{where}: not a known ARC site route")
            elif url != "/" and (url.endswith("/") or url.endswith(".html")):
                errors.append(f"{where}: trailing slash or .html")

print("\n".join(errors) or "links: ok")
sys.exit(1 if errors else 0)
