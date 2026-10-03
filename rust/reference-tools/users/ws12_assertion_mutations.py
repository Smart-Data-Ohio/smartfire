#!/usr/bin/env python3
"""Replay every credited WS12 declaration against its recorded producer mutation.

Install all runtime-selected recipes, compile once, run exact existing tests, then
restore byte-identical production sources. No vector or assertion is modified.
The receipt includes activations, assertion sites and full raw summaries. A failure
alone is not coverage: review its declared assertion before updating the ledger.
"""
import argparse
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor
from concurrent.futures import Future
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import threading

ROOT = Path(__file__).resolve().parents[3]
CATALOG = ROOT / "rust/plans/ws12-assertion-mutations.json"
HELPER = '''
fn ws12_coverage_mutant(name: &str) -> bool {
    let enabled = std::env::var("WS12_ASSERTION_MUTATION").as_deref() == Ok(name);
    if enabled { eprintln!("WS12_COVERAGE_HIT {name}"); }
    enabled
}
'''


def digest(data):
    return hashlib.sha256(data).hexdigest()


def combine(before, edits):
    """Combine recipes sharing an anchor without changing their disabled branch."""
    if len(edits) == 1:
        return edits[0]["after"]
    afters = [edit["after"] for edit in edits]
    if all(after.startswith(before) for after in afters):
        return before + "".join(after[len(before):] for after in afters)
    if all(after.endswith(before) for after in afters):
        return "".join(after[:-len(before)] for after in afters) + before
    if before == '    pub fn fail(error: impl Into<String>, status: u16) -> Self {\n        Self {':
        return ('    pub fn fail(error: impl Into<String>, status: u16) -> Self {\n'
                '        let error: String = error.into();\n' +
                "".join(re.search(r"        let status = .*?;\n", after).group()
                        for after in afters) + '        Self {')
    field = re.fullmatch(r"(\s*[a-z_]+: )(.*),", before)
    if field and all(after.startswith(field[1]) and after.endswith(",") for after in afters):
        value = field[2]
        for edit in reversed(edits):
            expression = edit["after"][len(field[1]):-1]
            value = ('if std::env::var("WS12_ASSERTION_MUTATION").as_deref() == Ok("'
                     + edit["mode"] + '") { ' + expression + ' } else { ' + value + ' }')
        return field[1] + value + ','
    if before.startswith("if ") and before.endswith(" {"):
        conditions = [after[3:-2] for after in afters]
        assert all(after.startswith("if ") and after.endswith(" {") for after in afters)
        value = before[3:-2]
        for edit, condition in reversed(list(zip(edits, conditions))):
            # Selection itself does not count as activation. The recipe's real
            # short-circuited producer gate records activation inside its branch.
            value = ('if std::env::var("WS12_ASSERTION_MUTATION").as_deref() == Ok("'
                     + edit["mode"] + '") { ' + condition + ' } else { ' + value + ' }')
        return "if (" + value + ") {"
    raise AssertionError(f"incompatible overlapping recipes: {before!r}")


def install(catalog, scratch):
    backup = scratch / "backups"
    assert not backup.exists(), "refuse overwriting previous mutation backups"
    grouped = defaultdict(lambda: defaultdict(list))
    for mutation in catalog["mutations"]:
        for edit in mutation["edits"]:
            grouped[edit["file"]][edit["before"]].append({**edit, "mode": mutation["key"]})
    outputs = {}
    for file, anchors in grouped.items():
        path = ROOT / file
        original = path.read_text()
        assert "WS12_ASSERTION_MUTATION" not in original
        source = original
        for before, edits in anchors.items():
            count = edits[0]["occurrences"]
            assert all(edit["occurrences"] == count for edit in edits)
            assert original.count(before) == count, (file, before, "source drift")
            assert source.count(before) == count, (file, before, "overlapping replacements")
            source = source.replace(before, combine(before, edits))
        if file.endswith(".rs"):
            source += HELPER
        outputs[file] = (path.read_bytes(), source.encode())
    backup.mkdir(parents=True)
    manifest = []
    for index, (file, (original, source)) in enumerate(outputs.items()):
        (backup / str(index)).write_bytes(original)
        manifest.append({"file": file, "index": index, "original": digest(original),
                         "instrumented": digest(source)})
    (backup / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    for file, (_, source) in outputs.items():
        (ROOT / file).write_bytes(source)
    print(f"WS12_COVERAGE_INSTALL {len(outputs)} production inputs; "
          f"{len(catalog['mutations'])} modes; vectors/assertions unchanged")


def restore(scratch):
    backup = scratch / "backups"
    manifest = json.loads((backup / "manifest.json").read_text())
    for row in manifest:
        assert digest((ROOT / row["file"]).read_bytes()) == row["instrumented"], \
            f"refuse overwriting edited/newly merged source: {row['file']}"
        assert digest((backup / str(row["index"])).read_bytes()) == row["original"]
    for row in manifest:
        (ROOT / row["file"]).write_bytes((backup / str(row["index"])).read_bytes())
    print(f"WS12_COVERAGE_RESTORE {len(manifest)} production inputs byte-identical")


def run(catalog, args):
    logs = args.scratch / "logs" / args.action
    logs.mkdir(parents=True, exist_ok=True)
    (args.scratch / "tmp").mkdir(parents=True, exist_ok=True)
    binaries = {}
    listed = {}
    for kind, stem in [("db", "campfire_db"), ("app", "campfire")]:
        choices = [path for path in (args.target / "debug/deps").glob(stem + "-*")
                   if path.is_file() and os.access(path, os.X_OK) and "." not in path.name
                   and any((args.target / "debug/.fingerprint" / path.name).glob("test-*.json"))]
        assert len(choices) == 1, (kind, choices)
        binaries[kind] = choices[0]
        listed[kind] = [line[:-6] for line in subprocess.check_output(
            [str(choices[0]), "--list"], text=True).splitlines() if line.endswith(": test")]
    rows = [row for row in catalog["declarations"] if not args.declaration or row["id"] in args.declaration]
    assert rows and (not args.declaration or set(args.declaration) <= {row["id"] for row in rows}), "unknown/empty declaration selection"
    baseline_cache = {}
    cache_lock = threading.Lock()

    def probe(row):
        groups = defaultdict(list)
        tests = row["previous_tests"] if args.previous_tests else row["tests"]
        for test in tests:
            kind = "db" if "/crates/db/" in test["rust_file"] else "app"
            names = [name for name in listed[kind] if name.rsplit("::", 1)[-1] == test["rust_test"]]
            assert len(names) == 1, test
            groups[kind].extend(names)
        result = {key: row[key] for key in ["id", "file", "line", "test", "mutation"]}
        result["groups"] = []
        for kind, names in groups.items():
            cache_key = (kind, tuple(dict.fromkeys(names)))
            cached = None
            if args.action == "baseline":
                with cache_lock:
                    cached = baseline_cache.get(cache_key)
                    if cached is None:
                        baseline_cache[cache_key] = Future()
                if cached is not None:
                    result["groups"].append(cached.result())
                    continue
            try:
                env = os.environ.copy()
                env.update(CI="1", RUST_TEST_THREADS="1", CABLE_TEST_PORT_RANGE="53420-53449",
                           MAIL_TEST_PORT_RANGE="53400-53419", GITHUB_TEST_PORT_RANGE="53450-53499",
                           TMPDIR=str(args.scratch / "tmp"),
                           LD_LIBRARY_PATH=str(ROOT / ".scratch/rails-media/native-libs"))
                env["PATH"] = str(ROOT / ".scratch/rails-media/usr/bin") + ":" + env["PATH"]
                if args.action == "run":
                    env["WS12_ASSERTION_MUTATION"] = row["mutation"]
                else:
                    env.pop("WS12_ASSERTION_MUTATION", None)
                command = (["bwrap", "--bind", "/", "/", "--unshare-net"] if kind == "app" else []) + [
                    str(binaries[kind]), "--exact", *dict.fromkeys(names), "--test-threads=1", "--nocapture"]
                path = logs / f"{row['id']}-{kind}.log"
                with path.open("w") as output:
                    done = subprocess.run(command, cwd=ROOT / f"rust/crates/{'db' if kind == 'db' else 'campfire'}",
                                          env=env, stdout=output, stderr=subprocess.STDOUT, timeout=600)
                text = path.read_text()
                summaries = re.findall(r"^test result:.*$", text, re.M)
                assert summaries, (row["id"], "no executed test summary", text[-1000:])
                count = re.search(r"(\d+) passed; (\d+) failed; (\d+) ignored;", summaries[-1])
                assert count and int(count[1]) + int(count[2]) == len(set(names)) and int(count[3]) == 0, \
                    (row["id"], "all selected assertions must run", summaries)
                group = {"kind": kind, "tests": list(dict.fromkeys(names)), "exit": done.returncode,
                         "hits": text.count("WS12_COVERAGE_HIT " + row["mutation"]),
                         "assertion_failed": "assertion `" in text or "assertion failed:" in text,
                         "summaries": summaries,
                         "panics": re.findall(r"^.*panicked at.*(?:\n.*){0,2}", text, re.M),
                         "log": str(path.relative_to(ROOT))}
                result["groups"].append(group)
            except BaseException as error:
                if args.action == "baseline":
                    baseline_cache[cache_key].set_exception(error)
                raise
            if args.action == "baseline":
                baseline_cache[cache_key].set_result(group)
        result["hits"] = sum(group["hits"] for group in result["groups"])
        result["rejected"] = any(group["exit"] for group in result["groups"])
        print(f"WS12_COVERAGE_PROBE {row['id']} mode={row['mutation']} "
              f"hits={result['hits']} rejected={result['rejected']}", flush=True)
        return result

    with ThreadPoolExecutor(max_workers=args.workers) as executor:
        results = list(executor.map(probe, rows))
    receipt = args.scratch / f"{args.action}-results.json"
    receipt.write_text(json.dumps(results, indent=2) + "\n")
    print(f"WS12_COVERAGE_CAMPAIGN {len(results)} declarations; "
          f"{sum(r['hits'] > 0 for r in results)} activated; "
          f"{sum(r['rejected'] for r in results)} rejected; "
          f"{sum(not r['rejected'] for r in results)} survived")
    if args.action == "baseline" and any(r["rejected"] for r in results):
        raise SystemExit("baseline failed; do not credit this campaign")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["install", "restore", "baseline", "run"])
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--target", type=Path)
    parser.add_argument("--declaration", action="append")
    parser.add_argument("--previous-tests", action="store_true",
                        help="reproduce the two reviewer controls against the old credited test sets")
    parser.add_argument("--workers", type=int, default=4, choices=range(1, 5))
    args = parser.parse_args()
    args.scratch = args.scratch.resolve()
    if args.target:
        args.target = args.target.resolve()
    catalog = json.loads(CATALOG.read_text())
    if args.action == "install":
        if args.declaration:
            selected = [row for row in catalog["declarations"] if row["id"] in args.declaration]
            assert set(args.declaration) == {row["id"] for row in selected}, "unknown declaration"
            modes = {row["mutation"] for row in selected}
            catalog = {**catalog, "mutations": [m for m in catalog["mutations"] if m["key"] in modes]}
        install(catalog, args.scratch)
    elif args.action == "restore":
        restore(args.scratch)
    else:
        assert args.target, "test binary directory required"
        if args.previous_tests:
            assert args.declaration and all("previous_tests" in row for row in catalog["declarations"]
                                           if row["id"] in args.declaration)
        run(catalog, args)
