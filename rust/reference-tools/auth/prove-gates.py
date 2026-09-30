#!/usr/bin/env python3
"""Run real tests against deliberate defects; always restore each source file."""
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
SCRATCH = ROOT.parent / ".scratch/auth/mutations"
SCRATCH.mkdir(parents=True, exist_ok=True)


def prove(name, path, old, new, package, test):
    source = ROOT / path
    original = source.read_text()
    assert original.count(old) == 1, f"{name}: injection anchor changed"
    try:
        source.write_text(original.replace(old, new))
        result = subprocess.run(
            ["cargo", "test", "-j", "4", "-p", package, test],
            cwd=ROOT, env=os.environ, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        )
        (SCRATCH / f"{name}.log").write_text(result.stdout)
        summaries = [line for line in result.stdout.splitlines() if line.startswith("test result:")]
        assert result.returncode != 0 and summaries and "FAILED" in summaries[0], result.stdout
        print(f"{name}: {summaries[0]}", flush=True)
    finally:
        source.write_text(original)


TOTP = "crates/rails_compat/src/totp.rs"
COOKIES = "crates/rails_compat/src/cookies.rs"
prove("totp-replay", TOTP, "step <= after.div_euclid(STEP_SECONDS)", "step < after.div_euclid(STEP_SECONDS)", "rails_compat", "totp::tests::rails_drift")
prove("totp-drift", TOTP, "checked_sub(STEP_SECONDS)", "checked_sub(2 * STEP_SECONDS)", "rails_compat", "totp::tests::rails_drift")
prove("totp-whitespace", TOTP, "!ruby_code_whitespace(*c)", "!c.is_whitespace()", "rails_compat", "totp::tests::rails_drift")
prove("cookie-purpose", COOKIES, 'format!("cookie.{name}")', 'format!("cookie.{}", { let _ = name; "two_factor_remember" })', "rails_compat", "totp::tests::rails_remember")
prove("cookie-tamper", COOKIES, "verify_signed(secrets, TWO_FACTOR_REMEMBER, raw, now)", 'Some({ let _ = (secrets, raw, now); "ws9-fixture-remember-token".into() })', "rails_compat", "totp::tests::rails_remember")
prove("cookie-expiry", COOKIES, "verify_signed(secrets, TWO_FACTOR_REMEMBER, raw, now)", "verify_signed(secrets, TWO_FACTOR_REMEMBER, raw, { let _ = now; Timestamp::from_second(0).unwrap() })", "rails_compat", "totp::tests::rails_remember")
print("WS9 security gates: 6 deliberate defects rejected")
