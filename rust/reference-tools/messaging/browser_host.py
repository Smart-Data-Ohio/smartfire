"""Build the existing TestApp boundary from reproducible, tools-only test inputs."""
import json
import importlib.util
import re
import shlex
from pathlib import Path
import subprocess

# Share CI's Rust lexer so comments, raw strings and generated-code strings do
# not look like compile-time includes. Unknown path expressions fail closed.
_spec = importlib.util.spec_from_file_location("rust_source", Path(__file__).resolve().parents[2] / "ci/ignored_tests.py")
_rust = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_rust)


def include_inputs(root, contents, tracked):
    inputs = set()
    for relative, content in contents.items():
        if relative.suffix != ".rs":
            continue
        source = root / relative

        def path_value(expression):
            expression = expression[:-1] if expression[-1:] == [","] else expression
            if len(expression) == 1:
                literal = expression[0]
                if literal.startswith('"'):
                    return json.loads(literal)
                raw = re.fullmatch(r'r(\#*)"(.*)"\1', literal, re.S)
                if raw:
                    return raw[2]
            if expression[:3] == ["concat", "!", "("] and expression[-1:] == [")"]:
                return "".join(path_value(part) for part in _rust.meta_items(expression[3:-1]) if part)
            if expression == ["env", "!", "(", '"CARGO_MANIFEST_DIR"', ")"]:
                return str(next(parent for parent in source.parents if (parent / "Cargo.toml").is_file()))
            raise RuntimeError(f"{relative}: unsupported compile-time include path: {' '.join(expression)}")

        items = _rust.tokens(content.decode())
        for index, item in enumerate(items):
            if item not in {"include_str", "include_bytes"} or items[index + 1:index + 3] != ["!", "("]:
                continue
            expression = items[index + 3:_rust.group_end(items, index + 2)]
            target = (source.parent / path_value(expression)).resolve()
            try:
                target = target.relative_to(root.resolve())
            except ValueError:
                raise RuntimeError(f"{relative}: compile-time include escapes the repository: {target}")
            if target not in tracked or not (root / target).is_file():
                raise RuntimeError(f"{relative}: tracked compile-time include missing: {target}")
            inputs.add(target)
    return inputs


def prepare_source(root):
    source = root / "rust"
    generated = root / ".scratch/ws8bm-browser-host/rust"
    # Copy source inputs, never a target, seed, or prior generated tree. Stable
    # paths permit Cargo caching, while every input is refreshed on each run.
    paths = subprocess.check_output(["git", "ls-files", "-z"], cwd=root, text=True).split("\0")
    tracked = {Path(path) for path in paths if path}
    wanted = [Path(path) for path in paths
              if path.startswith(("rust/crates/", "rust/vectors/", "rust/test-support/", "rust/reference-tools/views/agents_ui/")) or
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
    for relative in include_inputs(root, contents, tracked):
        contents.setdefault(relative, (root / relative).read_bytes())
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


def build_host(root, env):
    source = root / "rust"
    generated = prepare_source(root)
    # A generated package has the same Cargo identity as the workspace app.
    # Sharing its test executable with nextest replaces a running suite's
    # binary, despite different source roots. Keep this cache under target/.
    host_target = Path(env.get("CARGO_TARGET_DIR", source / "target")).resolve() / 'ws8bm-browser-host'
    host_env = dict(env, CAMPFIRE_REFERENCE=str(root), CARGO_TARGET_DIR=str(host_target))
    command = shlex.split(env.get("CAMPFIRE_CARGO", "mise exec rust@1.98.1 -- cargo")) + ["test", "--locked", "-j2",
               "--manifest-path", str(generated / "Cargo.toml"), "-p", "campfire", "--bin", "campfire",
               "--no-run", "--message-format=json"]
    result = subprocess.run(command, cwd=root, env=host_env, stdout=subprocess.PIPE, text=True)
    if result.returncode:
        for line in result.stdout.splitlines():
            entry=json.loads(line)
            if entry.get("reason")=="compiler-message":
                print(entry["message"].get("rendered",entry["message"]["message"]),flush=True)
        result.check_returncode()
    for line in result.stdout.splitlines():
        entry = json.loads(line)
        if (entry.get("reason") == "compiler-artifact" and entry.get("target", {}).get("name") == "campfire"
                and entry.get("profile", {}).get("test") and entry.get("executable")):
            return entry["executable"]
    raise RuntimeError("current-source browser test host executable missing")
