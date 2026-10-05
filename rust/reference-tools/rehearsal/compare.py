#!/usr/bin/env python3
"""Compare two smoke runs (smoke-<a>.json, smoke-<b>.json): statuses, key-element counts and
latency per path. Paths are printed with numeric ids replaced by :id, so the output carries no
production identifiers beyond the shape of the route.

Usage: compare.py WORK_DIR A B [--markdown]

--markdown adds a per-route table (median p50 across the route's pages, worst p95).
"""
import collections
import json
import re
import statistics
import sys

work, a, b = sys.argv[1:4]
markdown = "--markdown" in sys.argv
ra = {r["path"]: r for r in json.load(open(f"{work}/smoke-{a}.json"))["results"]}
rb = {r["path"]: r for r in json.load(open(f"{work}/smoke-{b}.json"))["results"]}
shape = lambda p: re.sub(r"/\d+", "/:id", p)

status_mismatch, count_diffs, rows = [], [], []
for path, x in ra.items():
    y = rb.get(path)
    if y is None:
        continue
    if x["status"] != y["status"] or x.get("location") != y.get("location"):
        status_mismatch.append((shape(path), x["status"], y["status"]))
    diffs = {}
    if x.get("counts") and y.get("counts"):
        for key in x["counts"]:
            if key == "bytes":
                continue
            if x["counts"][key] != y["counts"].get(key):
                diffs[key] = (x["counts"][key], y["counts"].get(key))
    if diffs:
        count_diffs.append((shape(path), diffs))
    rows.append((shape(path), x["status"], y["status"], x["p50_ms"], y["p50_ms"], x["p95_ms"], y["p95_ms"]))

print(f"paths compared: {len(rows)}; status/redirect mismatches: {len(status_mismatch)}; paths with element-count differences: {len(count_diffs)}")
for m in status_mismatch:
    print("  status", m)
for path, diffs in count_diffs:
    print("  counts", path, json.dumps(diffs))

p50a = [r[3] for r in rows]; p50b = [r[4] for r in rows]
print(f"median of per-path p50: {a} {statistics.median(p50a):.1f} ms, {b} {statistics.median(p50b):.1f} ms")
print(f"sum of per-path p50 (one pass over all paths): {a} {sum(p50a):.0f} ms, {b} {sum(p50b):.0f} ms")
if markdown:
    groups = collections.OrderedDict()
    for row in rows:
        groups.setdefault(row[0], []).append(row)
    print(f"\n| Route | Pages | {a} status | {b} status | {a} p50 ms | {b} p50 ms | {a} p95 ms | {b} p95 ms |")
    print("|---|---|---|---|---|---|---|---|")
    for route, group in groups.items():
        statuses = lambda i: "/".join(sorted({str(r[i]) for r in group}))
        med = lambda i: statistics.median(r[i] for r in group)
        print(f"| `{route}` | {len(group)} | {statuses(1)} | {statuses(2)} | {med(3):.1f} | {med(4):.1f} | {max(r[5] for r in group):.1f} | {max(r[6] for r in group):.1f} |")
