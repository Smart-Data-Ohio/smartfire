#!/usr/bin/env python3
"""Prove the native security tests detect bypasses; restore every source in finally."""
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root.parent / ".scratch/ws14g/mutations"
scratch.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, CARGO_TARGET_DIR=str(root / "target"), TMPDIR=str(scratch))
protocol = root / "crates/campfire/src/integrations/google/sign_in.rs"
identity = root / "crates/db/src/models/google_identity.rs"

def check(name, path, broken, package, test, assertion):
    original = path.read_text()
    assert broken != original, f"{name}: mutation did not change the source"
    try:
        path.write_text(broken)
        result = subprocess.run(["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j", "4", "-p", package, test, "--", "--nocapture"],
            cwd=root, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (scratch / f"{name}.log").write_text(result.stdout)
        lines = [line for line in result.stdout.splitlines() if line.startswith("test result:")]
        assert result.returncode == 101 and assertion in result.stdout and any("FAILED" in line for line in lines), result.stdout
        for line in lines:
            print(line, flush=True)
        print(f"security mutation {name}: rejected", flush=True)
    finally:
        path.write_text(original)

source = protocol.read_text()
check("state-bypass", protocol, source.replace("bool::from(verified.as_bytes().ct_eq(state.as_bytes()))", "let _ = (verified, state); true"),
    "campfire", "integrations::google::sign_in::tests::security_state", "state mismatch")
source = protocol.read_text()
start = source.index("    pub async fn verify(")
end = source.index("    async fn public_key_for(", start)
broken = source[:start] + '''    pub async fn verify(&self, token: &str, _nonce: &str, _purpose: &str, _now: i64) -> Result<Map<String, Value>, Error> {
        rails_compat::jwt::decode_unverified(token).map_err(|_| Error::Rejected("bad_token"))?
            .1.as_object().cloned().ok_or(Error::Rejected("bad_token"))
    }

''' + source[end:]
check("token-bypass", protocol, broken, "campfire", "integrations::google::sign_in::tests::security_verified", "security vector mismatches:")
source = identity.read_text()
check("email-trust-bypass", identity, source.replace("if !allowed || changed.is_some()", "if false && (!allowed || changed.is_some())"),
    "campfire_db", "models::google_identity::tests::security_", "expected admin_link_required")
print("Google sign-in security discrimination: 3 mutations rejected", flush=True)
source = identity.read_text()
check("timestamp-split", identity, source.replace("params![user_id,subject,email,domain,now,now]", "params![user_id,subject,email,domain,now,tx.now()]"),
    "campfire_db", "one_save_uses_one_timestamp", "one insert timestamp")
print("Google identity timestamp discrimination: 1 mutation rejected", flush=True)
