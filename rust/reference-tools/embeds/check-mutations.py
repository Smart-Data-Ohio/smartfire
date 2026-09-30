#!/usr/bin/env python3
"""Prove the security/parity regressions fail when their protection is broken."""
from pathlib import Path
import os
import subprocess

root = Path(__file__).resolve().parents[2]
env = os.environ.copy()
env.setdefault("TMPDIR", str(root.parent / ".scratch"))
env.setdefault("CARGO_TARGET_DIR", str(root / ".scratch/target"))
mutations = [
    ("net/redirect.rs", 'let relative = uri::parse(location).ok()?;', 'return uri::parse(location).ok().filter(Uri::is_http);\n    #[allow(unreachable_code)]\n    let relative = uri::parse(location).ok()?;', "ws15e_follows_relative_redirects"),
    ("opengraph/fetch.rs", 'for _ in 0..=max_redirects {', 'let _ = max_redirects;\n    for _ in 0..=MAX_REDIRECTS {', "ws15e_honors_zero_and_three_redirect_budgets"),
    ("opengraph/fetch.rs", 'match deadline {', 'let deadline = deadline.map(|_| Duration::from_secs(60));\n    match deadline {', "ws15e_deadline_covers_body_reads"),
    ("opengraph/fetch.rs", 'let ip = guard::resolve(net.resolver.as_ref(), url.host.as_deref().unwrap_or("")).await?;', 'let ip = "93.184.216.34".parse().unwrap();', "ws15e_rejects_private_redirect_before_dialing"),
    ("net/guard.rs", 'match ip {\n        IpAddr::V4', 'return false;\n    #[allow(unreachable_code)]\n    match ip {\n        IpAddr::V4', "ws15e_matches_our_rails_guard_corpus"),
    ("net/http.rs", 'if timer.as_mut().poll(cx).is_ready()', 'if false && timer.as_mut().poll(cx).is_ready()', "ws15e_write_timeout_bounds_a_stalled_transport"),
]
for name, old, new, test in mutations:
    path = root / "crates/campfire/src/integrations" / name
    source = path.read_text()
    assert source.count(old) == 1, (name, old)
    try:
        path.write_text(source.replace(old, new))
        result = subprocess.run(["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "-j", "4", "-p", "campfire", test, "--", "--nocapture"], cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        log = root.parent / ".scratch" / f"mutation-{test}.log"
        log.write_text(result.stdout)
        assert result.returncode != 0 and "test result: FAILED" in result.stdout, f"mutation survived or did not compile: {test}\n{result.stdout[-4000:]}"
        print(f"{test}: " + next(line for line in result.stdout.splitlines() if line.startswith("test result:")))
    finally:
        path.write_text(source)
print(f"WS15e mutation checks: {len(mutations)} detected, 0 survived")
