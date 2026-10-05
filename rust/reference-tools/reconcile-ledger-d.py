#!/usr/bin/env python3
"""Close the canonical D remainder only after its current browser receipts pass."""
import argparse
import copy
import json
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[2]
plans = root / "rust/plans"
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--nextest-list", type=Path, required=True)
parser.add_argument("--browser-log", type=Path, required=True)
parser.add_argument("--mutations", type=Path, required=True)
args = parser.parse_args()
names = [f"ledger-ws8br-ws17-ws11ui-d-{mode}-receipts.json"
         for mode in ["navigation", "members", "surfaces", "lifecycle"]]
command = ["python3", str(root / "rust/reference-tools/check-ledger-d-receipts.py"),
           "--nextest-list", str(args.nextest_list), "--browser-log", str(args.browser_log),
           "--mutations", str(args.mutations)]
for name in names:
    command += ["--manifest", str(plans / name)]
subprocess.run(command, check=True)

def load(name):
    return json.loads((plans / name).read_text())

def save(name, data):
    (plans / name).write_text(json.dumps(data, indent=2) + "\n")

manifests = {name: load(name) for name in names}
records = {r["id"]: (name, r) for name, m in manifests.items() for r in m["records"]}
broad = {key: value for key, value in records.items() if key.startswith("P")}
assert len(broad) == 104
assert all(r["record_status"] == "closed" for _, r in records.values())
branch = "rust/ledger-ws8br-ws17-ws11ui-d"
report = "rust/plans/ledger-ws8br-ws17-ws11ui-d-report.md"
remaining_name = "ledger-ws8br-ws17-ws11ui-remaining.json"
remaining = load(remaining_name)
receipts_name = "ledger-ws8br-ws17-ws11ui-receipts.json"
receipts = load(receipts_name)
if remaining["ws8br_broad_original_receipts"]:
    assert {r["id"] for r in remaining["ws8br_broad_original_receipts"]} == set(broad)
    remaining["history"].append({"date": "2026-10-05", "branch": branch,
        "previous_broad_pending": 104, "closed_broad_ids": sorted(broad),
        "closed_overlapping_criteria": 6, "muted_sequence_closed": 1,
        "calendar_sequence_closed": 1, "assertion_receipts": names,
        "excluded_geometry_dispositions_preserved": [r["id"] for r in remaining["excluded_geometry"]]})
    for ident, (name, record) in broad.items():
        receipts["ws8br_broad_closed_records"].append({
            "id": ident, "file": record["file"], "test": record["test"],
            "rust_tests": record["rust_tests"],
            "assertion_receipt": f"rust/plans/{name}#{ident}",
            "assertion_count": len(record["assertions"]),
            "private_helper_assertion_count": len(record.get("helper_expansion", [])),
            "setup_assertion_count": len(record.get("setup_expansion", [])),
            "execution": "D: current registered correctness wrapper passed twice at nextest -j 4; exact original assertions executed against pinned Rails and Rust in real Chromium."})
    for criterion in remaining["ws8br2_original_criteria"]:
        name, record = next((name, r) for name, r in broad.values()
                            if r["file"] == criterion["file"] and r["test"] == criterion["criterion"])
        item = copy.deepcopy(criterion)
        item.pop("reason")
        item["assertion_receipt"] = f"rust/plans/{name}#{record['id']}"
        receipts["ws8br2_criterion_closed_records"].append(item)
    for key, ident in [("ws8br_muted_browser", "muted-room-sequence"),
                       ("ws17_calendar_browser", "calendar-browser-sequence")]:
        name, r = records[ident]
        item = {"file": r["file"], "test": r["test"], "rust_tests": r["rust_tests"],
                "assertion_receipt": f"rust/plans/{name}#{ident}"}
        receipts[key + "_closed_records"] = [item]
    receipts["ws17_closed_records"].append({**receipts["ws17_calendar_browser_closed_records"][0],
        "rust_test": records["calendar-browser-sequence"][1]["rust_tests"][0]})
    for key in ["ws8br_broad_original_receipts", "ws8br2_original_criteria",
                "ws8br_muted_browser", "ws17_calendar_browser", "aggregate_mapping_gaps"]:
        remaining[key] = []
remaining.update(date="2026-10-05", partial=False, report=report,
                 disposition="All active receipt gaps closed; three pre-existing geometry exclusions retain their original dispositions.")
receipts["d_validation"] = {"report": report, "assertion_receipts": names,
    "registered_browser_tests": sorted({t for _, r in records.values() for t in r["rust_tests"]}),
    "new_broad_closures": 104, "overlapping_criteria": 6, "lifecycle_sequences": 2,
    "current_pin_additions": sum(len(m.get("current_pin_records", [])) for m in manifests.values()),
    "mutation_receipt": "rust/plans/ledger-ws8br-ws17-ws11ui-d-mutations.json"}
save(remaining_name, remaining)
save(receipts_name, receipts)

ws8_name = "ws8br-rails-cases.json"
ws8 = load(ws8_name)
for file in ws8["files"]:
    for row in file["declared_cases"]:
        ident = row.get("cutover_id")
        if ident not in broad:
            continue
        name, r = broad[ident]
        if row["cutover_disposition"] != "ported-current-assertion-receipt":
            row.setdefault("history", []).append({"date": "2026-10-05", "branch": branch,
                                                  "previous": copy.deepcopy(row)})
        row.update(cutover_disposition="ported-current-assertion-receipt", rust_tests=r["rust_tests"],
                   assertion_receipt=f"rust/plans/{name}#{ident}", assertion_count=len(r["assertions"]),
                   private_helper_assertion_count=len(r.get("helper_expansion", [])))
    if any(row.get("cutover_id") in broad for row in file["declared_cases"]):
        excluded = sum(row.get("cutover_id") in {r["id"] for r in remaining["excluded_geometry"]} for row in file["declared_cases"])
        file["status"] = f"Original per-assertion browser receipts complete; {excluded} existing geometry exclusions retained."
c = ws8["cutover_reconciliation"]
if c["broad_original_receipts_pending"]:
    c["history"].append({"date": "2026-10-05", "branch": branch, "previous": copy.deepcopy({k:v for k,v in c.items() if k != "history"})})
c.update(date="2026-10-05", partial=False, broad_original_receipts_pending=0,
         broad_original_assertion_receipts=337, current_report=report)
save(ws8_name, ws8)

ws17_name = "ws17-rails-test-inventory.json"
ws17 = load(ws17_name)
calendar = next(r for r in ws17["tests"] if r["file"] == "test/system/meeting_status_test.rb" and r["test"].startswith("opting in"))
if calendar["status"] != "ported-equivalent":
    calendar["history"].append({"date": "2026-10-05", "base": receipts["base"], "branch": branch,
                                "previous": copy.deepcopy({k:v for k,v in calendar.items() if k != "history"})})
calendar.update(status="ported-equivalent", rust_test=records["calendar-browser-sequence"][1]["rust_tests"][0],
    browser_receipt="rust/plans/ledger-ws8br-ws17-ws11ui-d-lifecycle-receipts.json#calendar-browser-sequence",
    evidence="Paired current real-browser opt-in, UI-enqueued refresh, actual busy intervals, shared injected clock advance and cleared rendered badge; twice through the registered wrapper.",
    seam="Tools-only injected clock/fixture bridge invokes the real refresh/cache and minute dispatcher; no production route or fabricated cache success.")
ws17["cutover_reconciliation"].update(date="2026-10-05", partial=False, closed_from_fifteen=15,
    remaining=0, current_named_passed=347, current_report=report)
save(ws17_name, ws17)

mappings_name = "ws8br-system-mappings.json"
mappings = load(mappings_name)
if mappings["partial"]:
    mappings["history"].append({"date": "2026-10-05", "branch": branch, "previous_partial": mappings["partial"]})
mappings["partial"] = False
mappings["cutover_reconciliation"].update(date="2026-10-05", partial=False, current_browser_report=report,
    reason="Exact named browser receipts close phone/header/member/pins aggregate gaps and the muted delivery sequence; exclusions retain their original dispositions.")
for item in mappings["files"]:
    if item["file"] == "test/system/sidebar_organize_test.rb":
        item["remaining"] = []
        item["additional_assertion_receipt"] = "rust/plans/ledger-ws8br-ws17-ws11ui-d-lifecycle-receipts.json#muted-room-sequence"
mappings["aggregate_assertion_receipts"] = ["rust/plans/ledger-ws8br-ws17-ws11ui-c-browser-receipts.json"] + ["rust/plans/" + n for n in names]
mappings["aggregate_disposition"] = "All active named declarations covered through exact per-assertion maps; three excluded geometry declarations remain outside the phase."
save(mappings_name, mappings)
print("D reconciliation: 104 broad + 6 overlapping criteria + 2 sequences closed; 0 active receipt gaps; 3 existing geometry exclusions preserved")
