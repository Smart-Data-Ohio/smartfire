#!/usr/bin/env python3
"""Reject deliberate sudo defects through the real seeded HTTP stack; always restore sources."""
from pathlib import Path
import os
import subprocess

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT.parent / ".scratch" / "sudo-mutations"
OUT.mkdir(parents=True, exist_ok=True)
CTRL = "crates/campfire/src/controllers/"
GATE = "    concerns::sudo::require_sudo_mode(c)?;\n"
cases = []
for name, path, occurrence, test in [
    ("join-code", "accounts/join_codes.rs", 0, "sudo_gate_and_replay"),
    ("custom-styles", "accounts/custom_styles.rs", 0, "sudo_applies"),
    ("bot-key", "accounts/bots/keys.rs", 0, "sudo_applies"),
    ("user-update", "accounts/users.rs", 0, "sudo_applies"),
    ("user-destroy", "accounts/users.rs", 1, "sudo_applies"),
    ("ban", "users/bans.rs", 0, "sudo_applies"),
    ("unban", "users/bans.rs", 1, "sudo_applies"),
    ("bot-create", "accounts/bots.rs", 0, "sudo_applies"),
    ("bot-webhook", "accounts/bots.rs", 1, "sudo_bot_webhook_gate"),
]:
    cases.append((name + "-gate-removed", CTRL + path, GATE, "", occurrence, test))
cases.extend([
    ("password-always-accepts", CTRL + "sudos.rs", "user.authenticate(&password)", "{ let _ = (user, password); true }", 0, "sudo_password_prompt"),
    ("totp-lockout-ignored", CTRL + "sudos.rs", "if credential.locked_out(tx.now())", "if false", 0, "sudo_totp_is_replay"),
    ("wrong-google-subject", CTRL + "sudos.rs", "== Some(verified_subject)", "!= Some(verified_subject)", 0, "sudo_google_seam"),
    ("wrong-google-member", CTRL + "sudos.rs", ".filter(|id| *id == flow_user_id)", ".filter(|_| { let _ = flow_user_id; true })", 0, "sudo_google_seam"),
    ("stale-google-login", CTRL + "sudos.rs", "c.now().as_second() - 330", "c.now().as_second() - 331", 0, "sudo_google_seam"),
    ("sudo-sixteen-minutes", "crates/campfire/src/concerns/session_keys.rs", "pub const SUDO_TIMEOUT: SignedDuration = SignedDuration::from_mins(15);", "pub const SUDO_TIMEOUT: SignedDuration = SignedDuration::from_mins(16);", 0, "sudo_requires_an_integer"),
    ("pending-request-reused", "crates/campfire/src/concerns/session_keys.rs", "session.remove(SUDO_PENDING_KEY)", "session.get(SUDO_PENDING_KEY).cloned()", 0, "sudo_gate_and_replay"),
    ("replay-csrf-missing", "crates/views/templates/sudos/continue.html", ".method(method)", ".method(method).authenticity_token(false)", 0, "sudo_gate_and_replay"),
])

for name, path, needle, replacement, occurrence, test in cases:
    source = ROOT / path
    original = source.read_text()
    positions = []
    start = 0
    while (position := original.find(needle, start)) >= 0:
        positions.append(position)
        start = position + len(needle)
    if len(positions) <= occurrence:
        raise RuntimeError(f"{name}: mutation anchor disappeared")
    position = positions[occurrence]
    try:
        source.write_text(original[:position] + replacement + original[position + len(needle):])
        env = dict(os.environ, TMPDIR=str(ROOT.parent / ".scratch" / "tmp"))
        result = subprocess.run(["cargo", "test", "--locked", "-j", "4", "-p", "campfire", "app::sudo_tests::" + test], cwd=ROOT, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (OUT / (name + ".log")).write_text(result.stdout)
        summaries = [line for line in result.stdout.splitlines() if line.startswith("test result:")]
        if result.returncode == 0 or not any("FAILED" in line for line in summaries):
            raise RuntimeError(f"{name}: defect did not produce a real failing test: see {OUT}")
        print(name + ": " + summaries[-1], flush=True)
    finally:
        source.write_text(original)
print(f"WS9 sudo security gates: {len(cases)} deliberate defects rejected", flush=True)
