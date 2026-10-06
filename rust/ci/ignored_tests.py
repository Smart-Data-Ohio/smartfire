#!/usr/bin/env python3
"""Fail closed on ignored correctness tests without an exact CI owner/selector."""
import argparse
import json
import os
from pathlib import Path
import re
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parent.parent


def tokens(source):
    """Lex attributes without treating comments or Rust literals as attributes."""
    pattern = re.compile(r'''\s+|//[^\n]*|/\*|(?:br|cr|r)(\#*)"|(?:b|c)?"(?:\\.|[^"\\])*"|(?:b)?'(?:\\.|[^'\\\n])'|(?:r\#)?[A-Za-z_][A-Za-z_0-9]*|.''', re.S)
    result = []
    offset = 0
    while offset < len(source):
        match = pattern.match(source, offset)
        value = match[0]
        offset = match.end()
        if value.isspace() or value.startswith("//"):
            continue
        if value == "/*":
            depth = 1
            while depth:
                comment = re.search(r"/\*|\*/", source[offset:])
                if not comment:
                    raise ValueError("unterminated Rust block comment")
                depth += 1 if comment[0] == "/*" else -1
                offset += comment.end()
            continue
        if match[1] is not None:
            end = source.find('"' + match[1], offset)
            if end < 0:
                raise ValueError("unterminated Rust raw string")
            value += source[offset:end + 1 + len(match[1])]
            offset = end + 1 + len(match[1])
        result.append(value)
    return result


def group_end(items, start):
    pairs = {"[": "]", "(": ")", "{": "}"}
    stack = [pairs[items[start]]]
    for index in range(start + 1, len(items)):
        value = items[index]
        if value in pairs:
            stack.append(pairs[value])
        elif value in pairs.values():
            if value != stack.pop():
                raise ValueError("unbalanced Rust attribute")
            if not stack:
                return index
    raise ValueError("unterminated Rust attribute")


def meta_items(items):
    parts, start, index = [], 0, 0
    while index < len(items):
        if items[index] in {"[", "(", "{"}:
            index = group_end(items, index)
        elif items[index] == ",":
            parts.append(items[start:index])
            start = index + 1
        index += 1
    return parts + [items[start:]]


def ignore_reasons(attribute):
    if not attribute:
        return []
    if attribute[0] == "ignore":
        if attribute == ["ignore"]:
            return [""]
        if len(attribute) != 3 or attribute[1] != "=":
            raise ValueError(f"unsupported ignore attribute: {attribute}")
        literal = attribute[2]
        if literal.startswith('"'):
            # Only a literal, nonescaped utility: prefix grants the utility exemption.
            return [literal[1:-1]]
        raw = re.fullmatch(r'r(\#*)"(.*)"\1', literal, re.S)
        if not raw:
            raise ValueError(f"unsupported ignore reason: {literal}")
        return [raw[2]]
    if attribute[0] == "cfg_attr" and attribute[1:2] == ["("]:
        # Conservatively classify every branch, including inactive/nested cfg_attr.
        return [reason for part in meta_items(attribute[2:-1])[1:]
                for reason in ignore_reasons(part)]
    return []


def inventory(root):
    found = {}
    # Every source directory, including tools-only copies; prune generated inputs/outputs.
    for directory, children, files in os.walk(root):
        children[:] = sorted(set(children) - {"target", "node_modules", ".cargo-home", ".scratch", ".ci", ".native", ".seed"})
        for name in sorted(files):
            if not name.endswith(".rs"):
                continue
            path = Path(directory) / name
            items = tokens(path.read_text())
            index = 0
            while index < len(items):
                if items[index:index + 2] != ["#", "["]:
                    index += 1
                    continue
                reasons = []
                while items[index:index + 2] == ["#", "["]:
                    end = group_end(items, index + 1)
                    reasons.extend(ignore_reasons(items[index + 2:end]))
                    index = end + 1
                if not reasons:
                    continue
                # Attributes bind to the immediately following item, regardless of lines.
                while index < len(items) and items[index] in {"pub", "async", "unsafe", "extern", "const", "("}:
                    if items[index] == "(":
                        index = group_end(items, index)
                    index += 1
                    if items[index:index + 1] and items[index].startswith('"'):
                        index += 1
                if items[index:index + 1] != ["fn"]:
                    raise ValueError(f"cannot resolve ignored function in {path}")
                key = (path.relative_to(root).as_posix(), items[index + 1].removeprefix("r#"))
                if key in found:
                    raise ValueError(f"duplicate ignored function: {key}")
                found[key] = next((reason for reason in reasons if not reason.startswith("utility:")), reasons[0])
    return found


def workflow_suites(workflow):
    """Map every correctness matrix suite to its shard list ("k/N"; empty means one job).

    A suite may appear in several matrices (for example two parts of one suite), but each
    matrix's shard list must be exactly 1/N..N/N so no slice of a suite is left unscheduled.
    """
    if 'bash rust/ci/correctness.sh "$SUITE"' not in workflow:
        raise ValueError("correctness runner is not invoked by the workflow")
    jobs = {}
    for match in re.finditer(r"suite: \[([^\]]+)\](?:\s*\n\s*shard: \[([^\]]+)\])?", workflow):
        shards = [item.strip().strip("'\"") for item in match[2].split(",")] if match[2] else []
        if shards and shards != [f"{index}/{len(shards)}" for index in range(1, len(shards) + 1)]:
            raise ValueError(f"shards must be 1/N..N/N: {shards}")
        for name in match[1].split(","):
            jobs.setdefault(name.strip(), []).append(shards)
    if not jobs:
        raise ValueError("correctness runner is not invoked by the workflow")
    return jobs


def check(root, manifest, workflow):
    found = inventory(root)
    owners = {}
    jobs = workflow_suites(workflow)
    for suite, records in manifest.items():
        if suite not in jobs:
            raise ValueError(f"missing CI job for ignored suite: {suite}")
        for record in records:
            key = (record["path"], record["test"].split("::")[-1])
            if key in owners:
                raise ValueError(f"multiple CI owners: {key}")
            if key not in found or found[key].startswith("utility:"):
                raise ValueError(f"stale correctness selector: {key}")
            owners[key] = suite
    for key, reason in found.items():
        if not reason.startswith("utility:") and key not in owners:
            raise ValueError(f"ignored correctness test has no CI job: {key}")
        if reason.startswith("utility:") and not reason.removeprefix("utility:").strip():
            raise ValueError(f"utility needs a reason: {key}")
    return len(owners), len(found) - len(owners)


def compiled_inventory(document):
    """Read nextest's compiler-expanded libtest discovery, without source guesses."""
    found = set()
    for suite in document["rust-suites"].values():
        if suite["status"] != "listed":
            raise ValueError(f"test binary was not enumerated: {suite['binary-id']}")
        for name, case in suite["testcases"].items():
            if case["ignored"]:
                if case["filter-match"]["status"] != "matches":
                    raise ValueError(f"ignored discovery unexpectedly filtered {name}")
                key = (suite["package-name"], suite["binary-name"], name)
                if key in found:
                    raise ValueError(f"duplicate compiled ignored test: {key}")
                found.add(key)
    return found


def check_compiled(document, manifest, utilities):
    correctness = [record for records in manifest.values() for record in records]
    expected = {(record["package"], record["binary"], record["test"])
                for record in correctness + utilities}
    if len(expected) != len(correctness) + len(utilities):
        raise ValueError("duplicate compiler inventory classification")
    found = compiled_inventory(document)
    if found != expected:
        raise ValueError(f"compiled ignored inventory mismatch: unclassified={sorted(found - expected)}, missing={sorted(expected - found)}")
    print(f"Compiler ignored-test guard: {len(correctness)} CI correctness tests, {len(utilities)} utilities; 0 unclassified")


def parse_shard(value):
    match = re.fullmatch(r"([1-9][0-9]*)/([1-9][0-9]*)", value or "")
    if not match or int(match[1]) > int(match[2]):
        raise argparse.ArgumentTypeError(f"expected K/N with 1 <= K <= N, got {value!r}")
    return int(match[1]), int(match[2])


def shard(records, selected):
    """Deterministically split a suite's records into N disjoint shards; return shard K.

    Longest-first by the optional observed "seconds" (default 1), each record going to the
    least-loaded shard (then fewest tests, then lowest index). Every record lands in exactly
    one shard, and no shard is empty.
    """
    index, count = selected
    if count > len(records):
        raise ValueError(f"{count} shards for {len(records)} selected tests would leave a shard empty")
    loads = [0.0] * count
    owned = [[] for _ in range(count)]
    for record in sorted(records, key=lambda r: (-r.get("seconds", 1), r["package"], r["binary"], r["test"])):
        target = min(range(count), key=lambda shard: (loads[shard], len(owned[shard]), shard))
        loads[target] += record.get("seconds", 1)
        owned[target].append(record)
    return owned[index - 1]


def expression(records):
    return " or ".join(f'(package(={r["package"]}) and binary(={r["binary"]}) and test(={r["test"]}))' for r in records)


def verify_junit(records, paths):
    """Require exactly the selected tests to have passed across the given receipts (shards)."""
    paths = [paths] if isinstance(paths, (str, Path)) else list(paths)
    cases = [case for path in paths for case in ET.parse(path).getroot().findall(".//testcase")]
    actual = [case.attrib["name"] for case in cases if case.find("skipped") is None]
    expected = sorted(record["test"] for record in records)
    if sorted(actual) != expected or any(case.find("failure") is not None or case.find("error") is not None for case in cases):
        raise ValueError(f"expected exactly {expected}; executed {actual}; see {', '.join(map(str, paths))}")
    print(f"Ignored correctness receipt: {len(actual)} passed, 0 failed, 0 selected tests skipped")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--filter", choices=["database", "acme", "browsers", "livekit", "messaging"])
    parser.add_argument("--package", action="store_true")
    parser.add_argument("--shard", type=parse_shard, help="K/N: only this shard of the suite's tests")
    parser.add_argument("--junit", type=Path, action="append", help="receipt; repeat to verify the union of shards")
    parser.add_argument("--nextest-list", type=Path)
    args = parser.parse_args()
    manifest = json.loads((ROOT / "ci/ignored-tests.json").read_text())
    correctness, utilities = check(ROOT, manifest, (ROOT.parent / ".github/workflows/rust.yml").read_text())
    if args.nextest_list:
        utilities = json.loads((ROOT / "ci/ignored-utilities.json").read_text())
        found = inventory(ROOT)
        for record in utilities:
            if not found.get((record["path"], record["test"].split("::")[-1]), "").startswith("utility:"):
                raise ValueError(f"compiled utility lacks a utility: reason: {record}")
        check_compiled(json.loads(args.nextest_list.read_text()), manifest, utilities)
    elif args.package:
        if not args.filter:
            parser.error("--package needs --filter")
        packages = {record["package"] for record in manifest[args.filter]}
        if len(packages) != 1:
            parser.error("each prerequisite suite must have one package")
        print(packages.pop())
    elif args.junit:
        if not args.filter:
            parser.error("--junit needs --filter")
        verify_junit(shard(manifest[args.filter], args.shard) if args.shard else manifest[args.filter], args.junit)
    elif args.filter:
        print(expression(shard(manifest[args.filter], args.shard) if args.shard else manifest[args.filter]))
    else:
        print(f"Ignored-test guard: {correctness} CI correctness tests, {utilities} utilities; 0 unowned")
