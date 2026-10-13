#!/usr/bin/env python3
"""Remove one OAuth guard at a time, require its negatives to fail, and restore it.

Run from the repository root with TMPDIR under ~/.cache and frozen seeds restored.
Only transient source edits are made; every probe restores the original bytes.
"""
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
assert Path.cwd().resolve() == ROOT, "Run from this checkout's root"
CACHE = Path(os.environ["TMPDIR"]).resolve()
assert CACHE.is_relative_to(Path.home() / ".cache"), "TMPDIR must be under ~/.cache"
CACHE.mkdir(parents=True, exist_ok=True)
ASSERTION = ROOT / "crates/app/src/oauth/assertion.rs"
CLIENT = ROOT / "crates/app/src/oauth/client.rs"
PROBES = [
    ("expired assertions", ASSERTION, [
        ("exp > now - SKEW && exp <= now + LIFETIME", "exp <= now + LIFETIME"),
        ("        if self.expires_at <= tx.now() {\n            return Ok(false);\n        }\n", ""),
    ], ["oauth_jwt_expired_without_iat_is_rejected", "oauth_jwt_expiry_at_skew_boundary_without_iat_is_rejected"]),
    ("absolute expiry", ASSERTION, [
        ("exp > now - SKEW && exp <= now + LIFETIME", "exp > now - SKEW"),
    ], ["oauth_jwt_expiry_too_far_without_iat_is_rejected"]),
    ("future iat", ASSERTION, [("iat <= now &&", "true &&")], ["oauth_jwt_future_iat_is_rejected"]),
    ("iat lifetime", ASSERTION, [
        ("(0..=LIFETIME).contains(&exp.saturating_sub(iat))", "exp.saturating_sub(iat) >= 0"),
    ], ["oauth_jwt_iat_lifetime_over_five_minutes_is_rejected"]),
    ("iat after exp", ASSERTION, [
        ("(0..=LIFETIME).contains(&exp.saturating_sub(iat))", "exp.saturating_sub(iat) <= LIFETIME"),
    ], ["oauth_jwt_iat_after_exp_within_skew_is_rejected"]),
    ("future nbf", ASSERTION, [("nbf <= now + SKEW", "true")], ["oauth_jwt_future_nbf_is_rejected"]),
    ("JWKS size", CLIENT, [
        (".is_none_or(|length| length <= LIMIT as u64)", ".is_none_or(|_| true)"),
        ("response.read_body(LIMIT)", "response.read_body(LIMIT * 2)"),
    ], ["oauth_jwt_jwks_content_length_over_limit_is_rejected", "oauth_jwt_jwks_chunked_body_over_limit_is_rejected"]),
    ("JWKS media type", CLIENT, [
        (".any(|media_type| t.eq_ignore_ascii_case(media_type))", ".any(|_| true)"),
    ], ["oauth_jwt_jwks_wrong_media_type_with_valid_keys_is_rejected"]),
    ("JWKS redirect", CLIENT, [("response.status == 200", "true")], ["oauth_jwt_jwks_redirect_with_valid_keys_is_rejected"]),
    ("RSA minimum size", ASSERTION, [
        ("n.len() * 8 - n[0].leading_zeros() as usize >= 2048", "n.len() * 8 - n[0].leading_zeros() as usize >= 1024"),
        ("signature::RSA_PKCS1_2048_8192_SHA256", "signature::RSA_PKCS1_1024_8192_SHA256_FOR_LEGACY_USE_ONLY"),
    ], ["oauth_jwt_correctly_signed_small_rsa_is_rejected"]),
    ("algorithm allowlist", ASSERTION, [("matches!(algorithm, \"RS256\" | \"ES256\")", "true")],
     ["oauth_jwt_header_claim_and_mechanism_refusals_leave_code_unconsumed"]),
]

env = dict(os.environ, CARGO_BUILD_JOBS="1")
for number, (name, source, changes, tests) in enumerate(PROBES, 1):
    original = source.read_bytes()
    mutated = original.decode()
    for before, after in changes:
        assert mutated.count(before) == 1, f"{name}: guard changed; update this probe"
        mutated = mutated.replace(before, after, 1)
    log_path = CACHE / f"oauth-guard-{number}.log"
    (CACHE / f"oauth-guard-{number}-original.rs").write_bytes(original)
    print(f"Checking {number}/{len(PROBES)}: {name}", flush=True)
    try:
        source.write_text(mutated)
        with log_path.open("wb") as log:
            result = subprocess.run([
                str(Path.home() / ".cargo/bin/cargo"), "nextest", "run", "-j", "2", "--locked",
                "--no-fail-fast", "-p", "campfire", "-E", " | ".join(f"test({test})" for test in tests),
            ], cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT)
    finally:
        source.write_bytes(original)
    output = log_path.read_text()
    failed = set(re.findall(r"^\s*FAIL\s+\[[^\n]+\]\s+\([^\n]+\)\s+\S+\s+\S*::(\w+)$", output, re.MULTILINE))
    assert result.returncode == 100 and failed == set(tests), f"{name}: expected test failures missing; see {log_path}"
    assert "assertion" in output and "failed" in output, f"{name}: failure must be behavioral"
    assert source.read_bytes() == original
    print(f"Confirmed {name}: {len(tests)} negatives failed; original guards restored", flush=True)
print("All negative guard probes passed; run the OAuth suite with the restored guards.", flush=True)
