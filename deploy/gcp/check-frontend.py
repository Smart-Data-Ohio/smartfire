#!/usr/bin/env python3
"""Read the public pages and the built SPA shell they land on after a healthy cutover."""
import argparse
from html.parser import HTMLParser
import json
from pathlib import Path
import re
import subprocess
import tempfile
from urllib.parse import urlsplit

# Each public path, and the path it must end on after redirects (None: wherever it lands). The
# retained sign-in and public pages redirect to the SPA's, which reference its built assets.
PAGES = {"/session/new": "/app/session/new", "/": None, "/about": "/app/about"}


class Assets(HTMLParser):
    def __init__(self):
        super().__init__()
        self.paths = set()

    def handle_starttag(self, tag, attributes):
        attributes = dict(attributes)
        path = attributes.get("src" if tag == "script" else "href", "")
        if tag in ("script", "link") and re.fullmatch(
            r"/app/assets/[A-Za-z0-9_-]+\.(?:js|css)", path
        ):
            self.paths.add(path)


def fetch(origin, path):
    with tempfile.TemporaryDirectory(prefix="frontend-check-") as scratch:
        body = Path(scratch) / "body"
        result = subprocess.run([
            "curl", "--silent", "--show-error", "--location", "--max-redirs", "4",
            "--connect-timeout", "5", "--max-time", "20", "--output", str(body),
            "--write-out", "%{http_code}\n%{url_effective}\n%{content_type}", "--url", origin + path,
        ], capture_output=True, text=True, check=True)
        status, url, content_type = result.stdout.split("\n", 2)
        if status != "200":
            raise ValueError(f"{path} returned {status}, expected 200")
        content = body.read_bytes()
        if not content:
            raise ValueError(f"{path} returned an empty body")
        return content, content_type, urlsplit(url).path


def check(origin, spa):
    receipt = {}
    verified = set()
    pages = dict(PAGES, **({"/app/offline.html": "/app/offline.html"} if spa else {}))
    for path, destination in pages.items():
        content, content_type, landed = fetch(origin, path)
        if destination is not None and landed != destination:
            raise ValueError(f"{path} ended at {landed}, expected {destination}")
        if "text/html" not in content_type:
            raise ValueError(f"{path} did not return HTML")
        assets = Assets()
        assets.feed(content.decode())
        paths = sorted(assets.paths)
        if {Path(asset).suffix for asset in paths} != {".js", ".css"}:
            raise ValueError(f"{path} is missing the built JS or CSS references")
        for asset in paths:
            if asset not in verified:
                _, asset_type, _ = fetch(origin, asset)
                expected = "javascript" if asset.endswith(".js") else "text/css"
                if expected not in asset_type:
                    raise ValueError(f"{asset} did not return {expected}")
                verified.add(asset)
        receipt[path] = {"status": 200, "location": landed, "assets": paths}
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("origin")
    parser.add_argument("--spa", action="store_true", help="also require the enabled SPA's built shell")
    args = parser.parse_args()
    try:
        print(json.dumps(check(args.origin.rstrip("/"), args.spa), sort_keys=True))
    except (OSError, ValueError, UnicodeError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"frontend check failed: {error}") from error


if __name__ == "__main__":
    main()
