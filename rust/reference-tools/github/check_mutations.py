#!/usr/bin/env python3
"""Inject known-bad security implementations and prove each selected test fails.

Run only in an isolated ws15g worktree with no concurrent builds. Sources are restored in finally.
"""
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root.parent / ".scratch/ws15g"
scratch.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, CARGO_TARGET_DIR=str(root / "target"), TMPDIR=str(scratch))
base = root / "crates/campfire/src/integrations/github"
mutations = [
    ("oauth-state", "oauth.rs", "return false;", "return true;", "github_state_rejects", 3),
    ("plaintext-access", "accounts.rs", "let access = crypto.encrypt(input.access_token);", "let access = input.access_token.to_string();", "accounts_encrypt_both_columns", 1),
    ("plaintext-refresh", "accounts.rs", "let refresh = input.refresh_token.map(|token| crypto.encrypt(token));", "let refresh = input.refresh_token.map(|token| token.to_string());", "accounts_encrypt_both_columns", 1),
    ("mention-injection", "client.rs", '.replace("@[", "@\\u{200b}[")', '.replace("@[", "@[")', "write_status_matrix", 1),
    ("owner-pat-side-effect", "accounts.rs", "&& self.usable(account.id).await?\n            && account.app_token()", "&& account.app_token()\n            && self.usable(account.id).await?", "agent_identity_checks_unreadable_owner_pat", 1),
    ("webhook-signature", "webhooks.rs", "!rails_compat::webhook::verify_github_signature(secret, raw, signature)", "false && !rails_compat::webhook::verify_github_signature(secret, raw, signature)", "webhook_security_", 2),
    ("webhook-secret", "../../controllers/github/webhooks.rs", "return Ok(c.head(StatusCode::SERVICE_UNAVAILABLE));", "return Ok(c.head(StatusCode::OK));", "webhook_security_missing_or_blank_secret", 1),
    ("webhook-dedupe", "webhooks.rs", "if inserted == 0 {\n        return Ok(false);", "if inserted == 0 {\n        return Ok(true);", "webhook_concurrent_duplicates_claim_and_enqueue_once", 1),
    ("claim-first-winner", "../action_claims.rs", '''"json_extract(agent_events.metadata, '$.status') = 'running'"''', '"1"', "github_claim_sweep_and_late_finish", 1),
    ("claim-cutoff", "../action_claims.rs", "created_at < ?", "created_at <= ?", "github_claim_sweep_fails_only_overdue", 1),
    ("fetch-retries", "jobs.rs", "RetryPolicy::no_retries()", "RetryPolicy::application_job()", "github_fetch_declares_one_attempt", 1),
    ("fetch-review-tie", "fetcher.rs", "if at > existing.1", "if at >= existing.1", "github_fetch_persisted_fields", 1),
    ("fetch-json-rescue", "fetcher.rs", "error.kind == ErrorKind::Fetch", "true", "github_fetch_persisted_fields", 1),
    ("agent-summary", "agent_actions.rs", "if action.action_name() != approval.action", "if false && action.action_name() != approval.action", "github_agent_rechecks_payload_and_linked_identity", 1),
    ("agent-identity", "agent_actions.rs", "if approval.account_id != Some(account.id)", "if false && approval.account_id != Some(account.id)", "github_agent_rechecks_payload_and_linked_identity", 1),
    ("agent-grant", "agent_actions.rs", "(deleted || !grant)", "(deleted || false && !grant)", "github_agent_rechecks_authority", 1),
    ("fetch-file-cap", "fetcher.rs", ".take(100)", ".take(usize::MAX)", "github_fetch_persisted_fields", 1),
]
for name, filename, before, after, test_filter, expected_failures in mutations:
    path = base / filename
    original = path.read_text()
    if before not in original:
        raise SystemExit(f"Mutation target disappeared: {name}")
    try:
        path.write_text(original.replace(before, after))
        command = ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j", "4", "-p", "campfire", test_filter, "--", "--nocapture"]
        run = subprocess.run(command, cwd=root, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (scratch / f"mutation-{name}.log").write_text(run.stdout)
        summaries = [line for line in run.stdout.splitlines() if line.startswith("test result:")]
        if run.returncode == 0 or not summaries or f"{expected_failures} failed;" not in summaries[-1]:
            print(run.stdout)
            raise SystemExit(f"Mutation was not rejected by the selected tests: {name}")
        print(f"{name}: {summaries[-1]}", flush=True)
    finally:
        path.write_text(original)
print(f"GitHub security mutations: {len(mutations)} rejected; 0 survived", flush=True)
