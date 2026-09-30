#!/usr/bin/env python3
"""Splits goldens.json (reference-tools/views/core/goldens.rb) into golden files: one per layout,
partial and page body, plus pages.json (their status and content type), helpers.json, facts.json
and cache_keys.json."""
import json
import os
import shutil
import sys

source, out = sys.argv[1], sys.argv[2]
with open(source, encoding="utf-8") as f:
    goldens = json.load(f)

shutil.rmtree(out, ignore_errors=True)
for group in ("layouts", "partials", "pages"):
    os.makedirs(os.path.join(out, group))


def write(path, text):
    with open(os.path.join(out, path), "w", encoding="utf-8", newline="") as f:
        f.write(text)


for name, html in goldens["layouts"].items():
    write(f"layouts/{name}.{'txt' if name.endswith('_text') else 'html'}", html)
for name, html in goldens["partials"].items():
    write(f"partials/{name}.html", html)
pages = {}
for name, page in goldens["pages"].items():
    extension = {"application/json": "json", "text/javascript": "js"}.get(page["content_type"].split(";")[0], "html")
    write(f"pages/{name}.{extension}", page["body"])
    pages[name] = {"path": page["path"], "status": page["status"], "content_type": page["content_type"], "file": f"{name}.{extension}"}
for name, value in (("pages", pages), ("helpers", goldens["helpers"]), ("facts", goldens["facts"]), ("cache_keys", goldens["cache_keys"])):
    write(f"{name}.json", json.dumps(value, indent=2, ensure_ascii=False) + "\n")
