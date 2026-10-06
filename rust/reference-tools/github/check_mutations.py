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
env = dict(os.environ, CI="1", CABLE_TEST_PORT_RANGE="51500-51549", CARGO_TARGET_DIR=str(root / "target"), TMPDIR=str(scratch))
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
    ("notifier-private-title", "notifier.rs", "!public && !subscription.verified", "false", "github_notifier_security", 1),
    ("notifier-mention", "notifier.rs", '.replace("@[", "@\\u{200b}[")', '.replace("@[", "@[")', "github_notifier_security", 1),
    ("notifier-dedupe", "subscriptions.rs", "ON CONFLICT(subscription_id,dedupe_key) DO NOTHING RETURNING id", "ON CONFLICT(subscription_id,dedupe_key) DO UPDATE SET updated_at=excluded.updated_at RETURNING id", "github_notifier_concurrent_duplicates", 1),
    ("notifier-deleted-room", "notifier.rs", "r.deleted_at IS NULL AND s.owner", "s.owner", "github_notifier_posts_claims", 1),
    ("notifier-invisible-reviewer", "notifier.rs", "AND (m.involvement IS NULL OR m.involvement!='invisible')", "", "github_notifier_posts_claims", 1),
    ("notifier-open-grant", "../../../../db/src/models/user.rs", "}, false)\n    }\n\n    fn create_with_open_room_grant", "}, true)\n    }\n\n    fn create_with_open_room_grant", "github_notifier_queue_failure", 1),
    ("notifier-broadcast-registration", "../../channels/sink.rs", "crate::integrations::github::notifier::MessageCreated::KIND =>", '"Github::Notifier#missing_broadcast" =>', "github_notifier_durable_handler", 1),

    ("message-reference-registration", "../../app.rs", "message_reference_syncs: vec![crate::integrations::github::references::sync]", "message_reference_syncs: vec![]", "github_message_reference_security", 1),
    ("reference-cap", "references.rs", "if triples.len() == 4", "if triples.len() == 8", "github_message_reference_security", 1),
    ("pr-private-agent-details", "pull_requests.rs", "if self.private == Some(false)", "if self.private == Some(true)", "github_pr_agent_payload_security", 1),
    ("card-broadcast-registration", "../../channels/sink.rs", "crate::integrations::github::pull_requests::CardUpdated::KIND =>", '"Github::PullRequest#missing_broadcast" =>', "github_pr_registered_card_callbacks", 1),
    ("viewer-reference-context", "pull_requests.rs", "message.room_id!=room_id||!conn.query_row", "message.room_id!=room_id||false && !conn.query_row", "github_viewer_card_security", 1),
    ("deleted-room-scope", "../../concerns.rs", "if room.deleted() { return Ok(None) }", "if false && room.deleted() { return Ok(None) }", "github_viewer_card_security", 1),
    ("pr-noop-timestamp", "pull_requests.rs", "if changed {", "if true {", "github_pr_identity_display_files", 1),
    ("thread-combined-error-cleanup", "threads.rs", 'e.0==vec![("github_pull_request_id","has already been taken".into())]', 'e.0.iter().any(|(field,_)|*field=="github_pull_request_id")', "github_pr_thread_uniqueness", 1),
    ("health-cutoff", "health.rs", "created_at>=?", "created_at>?", "github_health_counts", 1),
    ("subscription-member-authority", "../../controllers/github/subscriptions.rs", "if !user.can_administer(Some(room.creator_id), false)", "if false && !user.can_administer(Some(room.creator_id), false)", "github_subscription_http_security", 1),
    ("subscription-override-authority", "../../controllers/github/subscriptions.rs", "administrator_override: user.is_administrator()", "administrator_override: true", "github_subscription_http_status", 1),
    ("subscription-section-authority", "../../../../views/src/github/subscriptions.rs", "if !self.can_administer", "if false && !self.can_administer", "github_subscription_sections", 1),
    ("connection-sudo", "../../controllers/github/connections.rs", "concerns::sudo::require_sudo_mode(c)?;", "", "github_connections_security", 1),
    ("bot-link-admin", "../../controllers/github/connections.rs", "concerns::ensure_can_administer(c)?;", "", "github_connections_security", 1),
    ("app-callback-state", "../../controllers/github/connections.rs", "if !oauth::valid_state(", "if false && !oauth::valid_state(", "github_connections_security", 1),
    ("health-admin", "../../controllers/accounts/integrations_health.rs", "concerns::ensure_can_administer(c)?;", "", "integration_health_http_security", 1),
    ("discuss-reference", "threads.rs", "parent.thread_id.is_some()||!tx.conn().query_row", "parent.thread_id.is_some()||false && !tx.conn().query_row", "github_discuss_security", 1),
    ("human-write-own-token", "writes.rs", "accounts.write_client(token)", "accounts.write_client(String::from(\"fixture-wrong-token\"))", "github_write_http_results", 1),
    ("deactivation-registration", "../../app.rs", "user_deactivation_hooks: vec![crate::integrations::github::accounts::on_user_deactivation]", "user_deactivation_hooks: vec![]", "github_user_and_bot_deactivation", 1),

]
for name, filename, before, after, test_filter, expected_failures in mutations:
    path = base / filename
    original = path.read_text()
    if before not in original:
        raise SystemExit(f"Mutation target disappeared: {name}")
    try:
        path.write_text(original.replace(before, after))
        command = ["cargo", "test", "--locked", "-j", "4", "-p", "campfire", test_filter, "--", "--nocapture"]
        run = subprocess.run(command, cwd=root, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
        (scratch / f"mutation-{name}.log").write_text(run.stdout)
        summaries = [line for line in run.stdout.splitlines() if line.startswith("test result:")]
        if run.returncode == 0 or not summaries or f"{expected_failures} failed;" not in summaries[-1]:
            print(run.stdout)
            raise SystemExit(f"Mutation was not rejected by the selected tests: {name}")
        print(f"{name}: {summaries[-1]}", flush=True)
    finally:
        path.write_text(original)
print(f"GitHub security mutations: {len(mutations)} rejected; 0 survived", flush=True)
