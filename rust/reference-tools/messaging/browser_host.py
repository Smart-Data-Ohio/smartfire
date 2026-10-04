"""Build the existing TestApp boundary from reproducible, tools-only test inputs."""
import json
import shlex
from pathlib import Path
import subprocess


def prepare_source(root):
    source = root / "rust"
    generated = root / ".scratch/ws8bm-browser-host/rust"
    # Copy source inputs, never a target, seed, or prior generated tree. Stable
    # paths permit Cargo caching, while every input is refreshed on each run.
    paths = subprocess.check_output(["git", "ls-files", "rust", "public/500.html"], cwd=root, text=True).splitlines()
    wanted = [Path(path).relative_to("rust") for path in paths
              if path.startswith(("rust/crates/", "rust/vectors/", "rust/test-support/", "rust/reference-tools/views/agents_ui/")) or
              path in ("rust/Cargo.toml", "rust/Cargo.lock", "rust/rust-toolchain.toml", "rust/parity/.env.reference", "rust/parity/reference.sha",
                       "rust/reference-tools/messaging/older_provider_callbacks.rb")]
    expected = set(wanted)
    if generated.exists():
        for path in generated.rglob("*"):
            if path.is_file() and path.relative_to(generated) not in expected:
                path.unlink()
    for relative in wanted:
        path = generated / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        content = (source / relative).read_bytes()
        if relative == Path("crates/campfire/src/controllers/presenters/test_support.rs"):
            # -2 already provides this helper. Prefer it when present.
            if b"async fn ws8bm_browser_host_without_jobs()" not in content:
                content += (source / "reference-tools/messaging/browser-host.rs").read_bytes()
            content = content.replace(b"    let front = campfire_kit::front::FrontConfig::from_env();", b"    ws8bm_install_drive_client(&app);\n    let front = campfire_kit::front::FrontConfig::from_env();")
            content += (source / "reference-tools/messaging/browser-drive-client.rs").read_bytes()
        if not path.exists() or path.read_bytes() != content:
            path.write_bytes(content)
    # Main's cfg(test) modules also read tracked agents_ui cast/replay inputs
    # above. Preserve those paths in a cold copy, without copying any target.
    # Main's cfg(test) modules include this root-level Rails error fixture and
    # the provider callback above. Keep their original relative paths; neither
    # fixture may come from a pre-existing generated tree.
    if "public/500.html" not in paths:
        raise RuntimeError("tracked public/500.html test input missing")
    error_page = generated.parent / "public/500.html"
    error_page.parent.mkdir(parents=True, exist_ok=True)
    content = (root / "public/500.html").read_bytes()
    if not error_page.exists() or error_page.read_bytes() != content:
        error_page.write_bytes(content)
    return generated


def build_host(root, env):
    source = root / "rust"
    generated = prepare_source(root)
    host_env = dict(env, CAMPFIRE_REFERENCE=str(root),
                    CARGO_TARGET_DIR=str(Path(env.get("CARGO_TARGET_DIR", source / "target")).resolve()))
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
