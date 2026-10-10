#!/usr/bin/env python3
"""Read the public auth pages and built SPA shell after a healthy cutover."""
import argparse
from html.parser import HTMLParser
import json
from pathlib import Path
import re
import subprocess
import tempfile


class Assets(HTMLParser):
    def __init__(self):
        super().__init__()
        self.paths = set()

    def handle_starttag(self, tag, attributes):
        attributes = dict(attributes)
        path = attributes.get("src" if tag == "script" else "href", "")
        if tag in ("script", "link") and re.fullmatch(
            r"/(?:assets/auth-[0-9a-f]+|app/assets/[A-Za-z0-9_-]+)\.(?:js|css)", path
        ):
            self.paths.add(path)


def fetch(origin, path):
    with tempfile.TemporaryDirectory(prefix="frontend-check-") as scratch:
        body = Path(scratch) / "body"
        result = subprocess.run([
            "curl", "--silent", "--show-error", "--location", "--max-redirs", "4",
            "--connect-timeout", "5", "--max-time", "20", "--output", str(body),
            "--write-out", "%{http_code}\n%{content_type}", "--url", origin + path,
        ], capture_output=True, text=True, check=True)
        status, content_type = result.stdout.split("\n", 1)
        if status != "200":
            raise ValueError(f"{path} returned {status}, expected 200")
        content = body.read_bytes()
        if not content:
            raise ValueError(f"{path} returned an empty body")
        return content, content_type


def check(origin, spa):
    receipt = {}
    verified = set()
    for path in ["/session/new", "/"] + (["/app/offline.html"] if spa else []):
        content, content_type = fetch(origin, path)
        if "text/html" not in content_type:
            raise ValueError(f"{path} did not return HTML")
        assets = Assets()
        assets.feed(content.decode())
        prefix = "/app/assets/" if path == "/app/offline.html" else "/assets/auth-"
        paths = sorted(asset for asset in assets.paths if asset.startswith(prefix))
        if {Path(asset).suffix for asset in paths} != {".js", ".css"}:
            raise ValueError(f"{path} is missing the built JS or CSS references")
        for asset in paths:
            if asset not in verified:
                _, asset_type = fetch(origin, asset)
                expected = "javascript" if asset.endswith(".js") else "text/css"
                if expected not in asset_type:
                    raise ValueError(f"{asset} did not return {expected}")
                verified.add(asset)
        receipt[path] = {"status": 200, "assets": paths}
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
