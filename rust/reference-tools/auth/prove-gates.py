#!/usr/bin/env python3
"""Run real tests against deliberate defects; always restore each source file."""
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
SCRATCH = ROOT.parent / ".scratch/auth/mutations"
SCRATCH.mkdir(parents=True, exist_ok=True)
TEMP = ROOT.parent / ".scratch/tmp"
TEMP.mkdir(parents=True, exist_ok=True)
os.environ["TMPDIR"] = str(TEMP)


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

DB = "crates/db/src/models/two_factor.rs"
DEVICE = "crates/db/src/models/user_device.rs"
PREFIX = "tests::two_factor_test::"
prove("domain-missing", DB, "if tx.in_transaction() {\n        Ok(())", "if false {\n        Ok(())", "campfire_db", PREFIX)
prove("human-policy", DB, "self.is_active() && !self.is_bot()", "false", "campfire_db", PREFIX + "only_active_humans")
prove("stale-totp-replay", DB, "*self = Self::find(tx.conn(), self.id)?;\n        let Some(at)", "let Some(at)", "campfire_db", PREFIX + "concurrent_totp")
prove("confirmation-secret", DB, "let secret = decrypt_totp_secret(encryption, &setup.encrypted_secret)?;", "let secret = self.secret(encryption)?;", "campfire_db", PREFIX + "confirmation_rejects_wrong")
prove("backup-reuse", DB, "AND used_at IS NULL", "", "campfire_db", PREFIX + "backup_codes_are_single_use")
prove("backup-concurrent-reuse", DB, "AND used_at IS NULL", "", "campfire_db", PREFIX + "concurrent_backup_code")
prove("lockout-duration", DB, "1 | -2 => 60", "1 | -2 => 30", "campfire_db", PREFIX + "lockouts_escalate")
prove("locked-failures", DB, "if self.locked_out(tx.now())", "if false", "campfire_db", PREFIX + "failures_while_locked")
prove("lockout-concurrent", DB, "if self.locked_out(tx.now())", "if false", "campfire_db", PREFIX + "concurrent_failures")
prove("setup-expiry", DB, "filter(|s| !s.expired(now))", "filter(|s| { let _ = now; s.id > 0 })", "campfire_db", PREFIX + "setup_expiry")
prove("setup-session-binding", DB, "WHERE session_id = ?", "WHERE session_id = ? OR 1 = 1", "campfire_db", PREFIX + "setup_issuance")
prove("remember-user-binding", DB, "WHERE token_digest = ? AND user_id = ?", "WHERE token_digest = ? AND (? IS NOT NULL)", "campfire_db", PREFIX + "remembered_devices_reject")
prove("remember-server-expiry", DB, "if device.expired(tx.now())", "if false", "campfire_db", PREFIX + "remembered_devices_reject")
prove("remember-revoke-scope", DB, "WHERE user_id = ? AND id = ?", "WHERE (? IS NOT NULL) AND id = ?", "campfire_db", PREFIX + "remembered_device_revocation")
prove("user-device-missing", DEVICE, "let Some(device_id) =", 'return Err(crate::Error::Other("injected missing device model".into()));\n        let Some(device_id) =', "campfire_db", "tests::user_device_test::")
prove("collision-first-match", TOTP, "matched = Some(step.checked_mul(STEP_SECONDS).ok_or(InvalidTotp::Time)?);", "if matched.is_none() { matched = Some(step.checked_mul(STEP_SECONDS).ok_or(InvalidTotp::Time)?); }", "rails_compat", "totp::tests::newest_colliding")
prove("provisioning-issuer", TOTP, 'pub const ISSUER: &str = "Smartfire";', 'pub const ISSUER: &str = "Campfire";', "rails_compat", "totp::tests::provisioning_uris")
prove("base32-unicode", TOTP, "flat_map(char::to_uppercase)", "map(|c| c.to_ascii_uppercase())", "rails_compat", "totp::tests::rails_totp_codes")
prove("base32-binary-encoding", DB, 'decoded.encoding != "UTF-8" && !decoded.bytes.is_ascii()', 'false', "campfire_db", PREFIX + "unicode_base32_upcase")
print("WS9 security gates: 25 deliberate defects rejected")
