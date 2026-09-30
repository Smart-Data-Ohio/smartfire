#!/usr/bin/env python3
"""Prove the WS17 regressions discriminate, restoring source after each injected defect.

Run from the worktree root. Scratch output and TMPDIR stay under .scratch.
"""
import os
from pathlib import Path
import re
import subprocess
import sys

root = Path(__file__).resolve().parents[2]
scratch = root / ".scratch"
scratch.mkdir(exist_ok=True)
env = dict(os.environ, TMPDIR=str(scratch), CARGO_TARGET_DIR=str(root / "rust/target"),
           CAMPFIRE_TEST_REQUIRE_SEED="1", CABLE_TEST_PORT_RANGE="52400-52449", MAIL_TEST_PORT_RANGE="52400-52449")
cases = [
    ("policy-quiet-gate", "notification_policy.rs",
     "(!recipient.quiet_for_push(self.now) || self.dnd_exception)",
     "true",
     "ws17_policy_matches_rails_combinations"),
    ("reply-recipient", "push_subscription.rs",
     "Some(id) if message.reply_notify_author",
     "Some(id) if false",
     "ws17_replies_notify_only_opted_in_authors_and_merge_duplicate_scopes"),
    ("expired-status", "user_status_settings.rs",
     "self.custom_status_expires_at.is_some_and(|until| until <= now)",
     "self.custom_status_expires_at.is_some_and(|_| false)",
     "ws17_status_readers_match_rails_times_zones_and_dst"),
    ("prune-on-read", "workspace_presence_lease.rs",
     "let rows: Vec<(i64, Option<Timestamp>)>",
     'conn.execute("DELETE FROM workspace_presence_leases WHERE expires_at < ?", [now])?;\n        let rows: Vec<(i64, Option<Timestamp>)>',
     "ws17_expiry_is_inclusive_and_reads_never_prune"),
    ("valid-presence", "workspace_presence_lease.rs",
     "if !Self::identity_valid(tx.conn(), user_id, session_id)?",
     "if true",
     "tests::workspace_presence_lease_test::"),
    ("ghost-presence", "workspace_presence_lease.rs",
     "let sql = format!(",
     "return Ok(ids.iter().map(|id| (*id, Presence::Online)).collect());\n        let sql = format!(",
     "ws17_absent_users_and_expired_leases_are_not_returned"),
]
cases = [(name, f"rust/crates/db/src/models/{filename}", original, broken, test, "campfire_db")
         for name, filename, original, broken, test in cases]
cases += [
    ("settings-validation", "rust/crates/db/src/models/user_status_settings/writes.rs",
     "if !allowed.contains(&value)", "if false && !allowed.contains(&value)",
     "ws17_status_settings_validations_match_rails_and_leave_rows_unchanged", "campfire_db"),
    ("settings-keyword-rollback", "rust/crates/db/src/models/user_status_settings/writes.rs",
     "            self.save(tx)",
     '            tx.conn().execute_batch("RELEASE SAVEPOINT model_operation")?;\n            self.save(tx)',
     "an_invalid_settings_save_keeps_the_previous_keyword_list_atomically", "campfire_db"),
    ("settings-active-allowance", "rust/crates/db/src/models/dnd_allowed_user.rs",
     "if person.is_some_and", "if false && person.is_some_and",
     "allows_another_active_person_but_rejects_duplicates_self_bots_and_missing_people", "campfire_db"),
    ("settings-dnd-timer", "rust/crates/db/src/models/user_status_settings/writes.rs",
     "if !self.dnd_enabled || !self.manual_dnd_active(now)", "if true",
     "enabling_dnd_clears_an_expired_timer_and_preserves_a_live_one", "campfire_db"),
    ("settings-stale-write", "rust/crates/db/src/models/user_status_settings/writes.rs",
     ".zip(self.original_attributes.iter())", ".zip(Self::find(tx.conn(), self.user.id)?.original_attributes.iter())",
     "concurrent_settings_instances_only_write_the_fields_each_changed", "campfire_db"),
    ("presence-http-body", "rust/crates/campfire/src/controllers/users/presences.rs",
     'json!({"presences":presences})', 'json!({"presences":[]})',
     "ws17_presence_http_bodies_match_rails_vectors", "campfire"),
    ("web-push-tag", "rust/crates/campfire/src/integrations/web_push.rs",
     '"tag": self.tag', '"tag": null',
     "encodes_the_message_like_json_generate", "campfire"),
    ("service-worker-bytes", "rust/crates/campfire/src/controllers/pwa.rs",
     "pwa::SERVICE_WORKER_JS", '"/* wrong bytes */"',
     "ws17_service_worker_is_served_byte_identical_to_rails", "campfire"),
    ("next-form-bytes", "rust/crates/views/templates/users/profiles/_status.html",
     "Automatic shows you online", "Automatic always shows you offline",
     "ws17_owned_settings_html_matches_complete_rails_partials", "campfire_views"),
    ("next-controller-errors", "rust/crates/campfire/src/controllers/users/notification_settings.rs",
     "StatusCode::UNPROCESSABLE_ENTITY, submitted", "StatusCode::OK, submitted",
     "ws17_a_failed_save_keeps_the_previous_keywords", "campfire"),
    ("next-test-push-tag", "rust/crates/campfire/src/jobs/notifications.rs",
     'Some("test-notification".into())', 'Some("wrong-tag".into())',
     "ws17_durable_test_notification_decrypts_with_the_rails_payload_even_in_dnd", "campfire"),
]
if len(sys.argv)>1:
    cases=[case for case in cases if case[0].startswith(sys.argv[1])]
    assert cases, "no injection matches the supplied name prefix"
for name, filename, original, broken, test, package in cases:
    source = root / filename
    text = source.read_text()
    # Formatting may place a newline after the receiver in the expiry guard.
    if original not in text and name == "expired-status":
        match = re.search(r"self\s*\.custom_status_expires_at\s*\.is_some_and\(\|until\| until <= now\)", text)
        if match:
            original = match.group()
    assert original in text, f"injection anchor missing: {name}"
    log = scratch / f"{name}-injected.log"
    try:
        source.write_text(text.replace(original, broken, 1))
        with log.open("w") as output:
            result = subprocess.run([
                "mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j", "4",
                "--manifest-path", "rust/Cargo.toml", "-p", package, test, "--", "--test-threads=4",
            ], cwd=root, env=env, stdout=output, stderr=subprocess.STDOUT, check=False)
    finally:
        source.write_text(text)
    summary = re.search(r"^test result: FAILED\..*$", log.read_text(), re.MULTILINE)
    assert result.returncode != 0 and summary, f"injected defect escaped (or did not compile): {name}"
    print(f"{name}: detected")
    print(summary.group())
