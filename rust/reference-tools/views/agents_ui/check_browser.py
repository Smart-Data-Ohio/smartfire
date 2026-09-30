#!/usr/bin/env python3
"""Require a valid signed cookie to survive the HTTP test browser's expiry parser."""
from pathlib import Path
import os
import subprocess

root = Path(__file__).resolve().parents[3]
source = root / "crates/campfire/src/controllers/presenters/accounts/tests.rs"
original = source.read_text()
start = original.index("fn cookie_is_deleted(")
end = original.index("\n#[test]", start)
broken = '''fn cookie_is_deleted(cookie: &str) -> bool {
    cookie.to_ascii_lowercase().contains("max-age=0") || cookie.contains("1970")
}
'''
try:
    source.write_text(original[:start] + broken + original[end:])
    result = subprocess.run(
        ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked",
         "-j", "4", "-p", "campfire", "browser_keeps_valid_signed_cookies", "--", "--nocapture"],
        cwd=root, env={**os.environ, "CI": "1"}, capture_output=True, text=True)
    output = result.stdout + result.stderr
    assert result.returncode != 0 and "test result: FAILED." in output and "valid signature digits are not an expiry attribute" in output, f"browser regression escaped test\n{output}"
    print(next(line for line in output.splitlines() if line.startswith("test result:")))
    print("Browser cookie discrimination: valid signed-cookie regression rejected; source restored")
finally:
    source.write_text(original)
