"""Build the existing TestApp boundary from reproducible, tools-only test inputs."""
import json
import importlib.util
import os
import re
import shlex
from pathlib import Path
import subprocess

# Share CI's Rust lexer so comments, raw strings and generated-code strings do
# not look like compile-time includes. Unknown path expressions fail closed.
_spec = importlib.util.spec_from_file_location("rust_source", Path(__file__).resolve().parents[2] / "ci/ignored_tests.py")
_rust = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_rust)


def include_calls(content):
    """Return actual invocations, including macro bodies, but not Rust literals."""
    items = _rust.tokens(content.decode())
    for index, item in enumerate(items):
        # The terminal macro identifier is shared by bare and qualified paths.
        item = item.removeprefix("r#")
        if item not in {"include", "include_str", "include_bytes"} or items[index + 1:index + 2] != ["!"]:
            continue
        if items[index + 2:index + 3] not in [["("], ["["], ["{"]]:
            raise ValueError(f"invalid {item}! delimiter")
        yield item, items[index + 3:_rust.group_end(items, index + 2)]


def include_inputs(root, contents, tracked, *, build_outputs=None, pending=None,
                   generated_root=None, edges=None):
    """Resolve all includes and follow included Rust, failing on unknown paths.

    Only source preparation may defer OUT_DIR. build_host must subsequently
    resolve those calls against this Cargo invocation's build-script receipts.
    """
    root = root.resolve()
    build_outputs = build_outputs or {}
    generated_roots = tuple(Path(path).resolve() for path in build_outputs.values())

    def validate(target, source):
        base = generated_root if generated_root and target.is_relative_to(generated_root) else root
        if not target.is_file():
            label = target.relative_to(base) if target.is_relative_to(base) else target
            raise RuntimeError(f"{source}: tracked compile-time include missing: {label}")
        if any(target.is_relative_to(path) for path in generated_roots):
            return
        if not target.is_relative_to(base):
            raise RuntimeError(f"{source}: compile-time include escapes the repository: {target}")
        if target.relative_to(base) not in tracked:
            raise RuntimeError(f"{source}: tracked compile-time include missing: {target.relative_to(base)}")

    inputs = set()
    queue = [(root / relative, content, None) for relative, content in contents.items() if relative.suffix == ".rs"]
    visited = set()
    while queue:
        source, content, inherited_manifest = queue.pop()
        source = source.resolve()
        manifest = (inherited_manifest
                    or next((owner for owner, output in build_outputs.items() if source.is_relative_to(output)), None)
                    or next((parent for parent in source.parents if (parent / "Cargo.toml").is_file()), None))
        if (source, manifest) in visited:
            continue
        visited.add((source, manifest))
        relative = source.relative_to(root) if source.is_relative_to(root) else source
        needs_output = False

        def path_value(expression):
            nonlocal needs_output
            expression = expression[:-1] if expression[-1:] == [","] else expression
            if len(expression) == 1:
                literal = expression[0]
                if literal.startswith('"'):
                    # Rust additionally permits hexadecimal/unicode escapes and
                    # escaped newlines. Tokenize escapes so a literal \\x stays literal.
                    def escape(match):
                        value = match[0][1:]
                        if value.startswith("u{"):
                            return json.dumps(chr(int(value[2:-1].replace("_", ""), 16)))[1:-1]
                        if value.startswith("x"):
                            return json.dumps(chr(int(value[1:], 16)))[1:-1]
                        if value == "0":
                            return r"\u0000"
                        if value == "'":
                            return "'"
                        if "\n" in value:
                            return ""
                        return match[0]
                    return json.loads(re.sub(r"\\(?:u\{[\da-fA-F_]+\}|x[\da-fA-F]{2}|\r?\n\s*|.)", escape, literal))
                raw = re.fullmatch(r'r(\#*)"(.*)"\1', literal, re.S)
                if raw:
                    return raw[2]
            if len(expression) >= 4 and expression[1:2] == ["!"] and expression[2] in {"(", "[", "{"} and _rust.group_end(expression, 2) == len(expression) - 1:
                if expression[0] == "concat":
                    return "".join(path_value(part) for part in _rust.meta_items(expression[3:-1]) if part)
                if expression[0] == "env":
                    key = path_value(expression[3:-1])
                    if key == "CARGO_MANIFEST_DIR" and manifest:
                        return str(manifest)
                    if key == "OUT_DIR" and manifest:
                        if manifest in build_outputs:
                            return str(build_outputs[manifest])
                        if pending is not None and (manifest / "build.rs").is_file():
                            needs_output = True
                            return "/__cargo_output_pending__"
            raise RuntimeError(f"{relative}: unsupported compile-time include path: {' '.join(expression)}")

        try:
            calls = list(include_calls(content))
        except ValueError as error:
            raise RuntimeError(f"{relative}: {error}") from error
        for macro, expression in calls:
            needs_output = False
            try:
                value = path_value(expression)
                target = (source.parent / value).resolve()
            except (ValueError, OSError) as error:
                raise RuntimeError(f"{relative}: unsupported compile-time include path: {' '.join(expression)}: {error}") from error
            if needs_output:
                pending.append({"source": str(relative), "macro": macro, "expression": expression})
                continue
            # Check containment before existence so an escaping missing path is
            # reported as an escape rather than as a repository input.
            if not target.is_relative_to(root) and not any(target.is_relative_to(path) for path in generated_roots):
                raise RuntimeError(f"{relative}: compile-time include escapes the repository: {target}")
            validate(target, relative)
            inputs.add(target.relative_to(root) if target.is_relative_to(root) else target)
            if edges is not None:
                edges.append({"source": str(relative), "macro": macro, "target": str(target)})
            if macro == "include":
                queue.append((target, contents.get(target.relative_to(root), target.read_bytes()) if target.is_relative_to(root) else target.read_bytes(), manifest))
    return inputs


def prepare_source(root):
    source = root / "rust"
    generated = root / ".scratch/ws8bm-browser-host/rust"
    # Copy source inputs, never a target, seed, or prior generated tree. Stable
    # paths permit Cargo caching, while every input is refreshed on each run.
    paths = subprocess.check_output(["git", "ls-files", "-z"], cwd=root, text=True).split("\0")
    tracked = {Path(path) for path in paths if path}
    wanted = [Path(path) for path in paths
              if path.startswith(("rust/crates/", "rust/vectors/", "rust/test-support/", "rust/web/", "rust/fixtures/")) or
              path in ("rust/Cargo.toml", "rust/Cargo.lock", "rust/rust-toolchain.toml", "rust/parity/.env.reference", "rust/parity/reference.sha")]
    contents = {}
    for relative in wanted:
        content = (root / relative).read_bytes()
        if relative == Path("rust/crates/campfire/src/controllers/presenters/test_support.rs"):
            # -2 already provides this helper. Prefer it when present.
            if b"async fn ws8bm_browser_host_without_jobs()" not in content:
                content += (source / "reference-tools/messaging/browser-host.rs").read_bytes()
            content = content.replace(b"    let front = campfire_kit::front::FrontConfig::from_env();", b"    ws8bm_install_drive_client(&app);\n    let front = campfire_kit::front::FrontConfig::from_env();")
            content = content.replace(b"        app.booted.router.clone(),", b"        app.booted.router.clone().merge(ws8bm_attachment_job_router(&app)),")
            content += (source / "reference-tools/messaging/browser-drive-client.rs").read_bytes()
            content += (source / "reference-tools/messaging/browser-attachment-jobs.rs").read_bytes()
        contents[relative] = content
    pending = []
    for relative in include_inputs(root, contents, tracked, pending=pending):
        contents.setdefault(relative, (root / relative).read_bytes())
    contents[Path("rust/include-audit-pending.json")] = json.dumps(pending, indent=2).encode()
    if generated.parent.exists():
        for path in generated.parent.rglob("*"):
            if path.is_file() and path.relative_to(generated.parent) not in contents:
                path.unlink()
    for relative, content in contents.items():
        path = generated.parent / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        if not path.exists() or path.read_bytes() != content:
            path.write_bytes(content)
    return generated


def audit_build(root, generated, entries, host_target):
    """Audit generated code and every compiler dependency for workspace artifacts."""
    root = root.resolve()
    generated = generated.resolve()
    tracked = {Path(path) for path in subprocess.check_output(["git", "ls-files", "-z"], cwd=root, text=True).split("\0") if path}
    artifacts = [entry for entry in entries if entry.get("reason") == "compiler-artifact"
                 and Path(entry["manifest_path"]).is_relative_to(generated)]
    manifests = {entry["package_id"]: Path(entry["manifest_path"]).parent for entry in artifacts}
    outputs = {manifests[entry["package_id"]]: Path(entry["out_dir"]).resolve()
               for entry in entries if entry.get("reason") == "build-script-executed" and entry["package_id"] in manifests}
    if not artifacts:
        raise RuntimeError("compiler include audit: workspace artifact receipts missing")
    dependencies = set()
    for entry in artifacts:
        depfiles = set()
        for filename in map(Path, entry["filenames"]):
            stem = filename.stem.removeprefix("lib") if filename.suffix in {".rlib", ".rmeta", ".so"} else filename.name
            path = filename.with_name(stem + ".d")
            if path.is_file():
                depfiles.add(path)
            if "custom-build" in entry["target"]["kind"]:
                depfiles.update(filename.parent.glob("*.d"))
        if not depfiles:
            raise RuntimeError(f"{entry['target']['src_path']}: compiler dependency output missing")
        for depfile in depfiles:
            text = depfile.read_text().replace("\\\n", "")
            rule = next((line.split(": ", 1)[1] for line in text.splitlines() if ": " in line and not line.startswith("#")), None)
            if rule is None:
                raise RuntimeError(f"{depfile}: compiler dependency rule missing")
            dependencies.update((generated / path.replace("$$", "$")).resolve() for path in shlex.split(rule))
    contents = {path.relative_to(root): path.read_bytes() for path in generated.rglob("*.rs")}
    for path in dependencies:
        if path.suffix == ".rs" and any(path.is_relative_to(output) for output in outputs.values()):
            contents[path] = path.read_bytes()
    edges = []
    inputs = include_inputs(root, contents, tracked, build_outputs=outputs,
                            generated_root=generated.parent, edges=edges)
    output_roots = tuple(outputs.values())
    for path in dependencies:
        if not path.is_file():
            raise RuntimeError(f"compiler include audit: dependency missing: {path}")
        if any(path.is_relative_to(output) for output in output_roots):
            continue
        base = generated.parent if path.is_relative_to(generated.parent) else root
        if not path.is_relative_to(base) or path.relative_to(base) not in tracked:
            raise RuntimeError(f"compiler include audit: untracked dependency: {path}")
    def is_generated(path):
        return any((root / path).is_relative_to(output) for output in output_roots)

    generated_edges = [edge for edge in edges if is_generated(Path(edge["source"])) or is_generated(Path(edge["target"]))]
    for edge in generated_edges:
        if Path(edge["target"]) not in dependencies:
            raise RuntimeError(f"{edge['source']}: include absent from compiler dependency output: {edge['target']}")
    report = {"include_targets": sorted(map(str, inputs)), "generated_include_edges": generated_edges,
              "compiler_dependencies": sorted(map(str, dependencies)), "pending": []}
    (host_target / "include-audit.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def build_host(root, env):
    source = root / "rust"
    generated = prepare_source(root)
    # A generated package has the same Cargo identity as the workspace app.
    # Sharing its test executable with nextest replaces a running suite's
    # binary, despite different source roots. Keep this cache under target/.
    host_target = Path(env.get("CARGO_TARGET_DIR", source / "target")).resolve() / 'ws8bm-browser-host'
    host_env = dict(env)
    host_env["CARGO_TARGET_DIR"] = str(host_target)
    jobs = env.get("WS8BM_HOST_BUILD_JOBS", "2")  # parallelism only; the output is the same
    command = shlex.split(env.get("CAMPFIRE_CARGO", "cargo")) + ["test", "--locked", f"-j{jobs}",
               "--manifest-path", str(generated / "Cargo.toml"), "-p", "campfire", "--bin", "campfire",
               "--no-run", "--message-format=json"]
    result = subprocess.run(command, cwd=root, env=host_env, stdout=subprocess.PIPE, text=True)
    if result.returncode:
        for line in result.stdout.splitlines():
            entry=json.loads(line)
            if entry.get("reason")=="compiler-message":
                print(entry["message"].get("rendered",entry["message"]["message"]),flush=True)
        result.check_returncode()
    entries = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
    audit_build(root, generated, entries, host_target)
    for entry in entries:
        if (entry.get("reason") == "compiler-artifact" and entry.get("target", {}).get("name") == "campfire"
                and entry.get("profile", {}).get("test") and entry.get("executable")):
            return entry["executable"]
    raise RuntimeError("current-source browser test host executable missing")


if __name__ == "__main__":
    # CI builds and audits this host once, then hands the executable to every sharded
    # behaviour job (.github/workflows/rust.yml); behavior-check.py otherwise builds it.
    root = Path(__file__).resolve().parents[3]
    print(build_host(root, dict(os.environ)))
