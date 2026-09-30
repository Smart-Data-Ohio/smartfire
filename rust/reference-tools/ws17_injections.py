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
    ("profile-auth-atomic", "rust/crates/campfire/src/controllers/users/profiles.rs",
     "settings.save(tx)?;", 'settings.save(tx)?; tx.conn().execute_batch("COMMIT")?;',
     "ws17_profile_auth_audit_failure_rolls_back_the_earlier_appearance_save", "campfire"),
    ("profile-layout-snapshot", "rust/crates/campfire/src/controllers/presenters/view_context.rs",
     "apply_settings_preferences(&mut preferences, settings, app_now);", "let _ = settings;",
     "ws17_rejected_profile_layout_metadata_matches_loaded_unsaved_rails_values", "campfire"),
    ("profile-save-theme", "rust/crates/campfire/src/controllers/users/profiles.rs",
     "settings.theme = theme;", "settings.theme = theme; settings.theme = \"system\".into();",
     "ws17_update_saves_theme_and_time_zone", "campfire"),
    ("profile-private-endpoint", "rust/crates/campfire/src/controllers/users/push_subscriptions.rs",
     "resolved.insert(host, ip);", "resolved.insert(host, Some(\"142.250.123.45\".into())); let _ = ip;",
     "ws17_rejects_subscription_with_endpoint_resolving_to_private_ip", "campfire"),
    ("profile-unique-loser", "rust/crates/campfire/src/controllers/users/dnd_allowances.rs",
     "&& !error.is_record_not_unique()", "&& (true || !error.is_record_not_unique())",
     "ws17_unique_index_losing_star_redirects_success", "campfire"),
    ("profile-sound-metadata", "rust/crates/campfire/src/controllers/presenters/view_context.rs",
     "preferences.notification_sounds = sounds;", "preferences.notification_sounds = Default::default(); let _ = sounds;",
     "ws17_layout_carries_current_and_future_meeting_and_ooo_windows", "campfire"),
    ("keyword-priority", "rust/crates/db/src/models/activity_item/message_recorder.rs",
     "mentioned: mentioned.contains(&id)", "mentioned: false",
     "ws17_message_activity_matches_actual_rails_callbacks_and_scoped_candidates", "campfire_db"),
    ("keyword-read-state", "rust/crates/db/src/models/activity_item/message_recorder.rs",
     "ON CONFLICT(user_id,source_type,source_id) DO NOTHING", "ON CONFLICT(user_id,source_type,source_id) DO UPDATE SET read_at=NULL,handled_at=NULL",
     "ws17_same_message_recording_keeps_type_read_and_handled_state", "campfire_db"),
    ("keyword-atomic", "rust/crates/db/src/models/message.rs",
     "crate::ActivityItem::record_message(tx, &message)?;", "tx.conn().execute_batch(\"COMMIT\")?; crate::ActivityItem::record_message(tx, &message)?;",
     "ws17_keyword_http_insert_failure_rolls_back_message_index_and_jobs", "campfire"),
    ("named-policy-quiet", "rust/crates/db/src/models/notification_policy.rs",
     "(!recipient.quiet_for_push(self.now) || self.dnd_exception)", "true",
     "named_policy_test", "campfire_db"),
    ("named-policy-bot-inbox", "rust/crates/db/src/models/notification_policy.rs",
     "if !self.recipient.is_some_and(UserStatusSettings::active_human) || self.invisible()", "if self.invisible()",
     "ws17_named_policy_bots_and_deactivated_recipients_record_nothing", "campfire_db"),
    ("named-policy-query-ceiling", "rust/crates/db/src/models/notification_policy.rs",
     "let values: Vec<i64> =", 'conn.query_row("SELECT count(*) FROM dnd_allowed_users",[],|row|row.get::<_,i64>(0))?; let values: Vec<i64> =',
     "ws17_named_policy_dnd_exceptions_load_for_batch_in_one_query", "campfire_db"),
    ("named-status-deactivate", "rust/crates/db/src/models/user_status_settings.rs",
     "self.ooo_until = None;\n        self.ooo_note = None;\n        self.ooo_broadcast = None;", "// Keep stale loaded OOO fields",
     "ws17_named_out_of_office_deactivating_clears_the_manual_ooo_columns", "campfire_db"),
    ("named-status-expired-note", "rust/crates/db/src/models/user_status_settings.rs",
     "if self.manual_ooo_active(now)\n            && let Some(note)", "if let Some(note)",
     "ws17_named_out_of_office_the_note_shows_only_while_the_manual_ooo_is_active", "campfire_db"),
    ("named-status-meeting-dnd", "rust/crates/db/src/models/user_status_settings.rs",
     "&& !self.dnd_active(now)", "&& true",
     "named_calendar_status_test", "campfire_db"),
    ("push-reminder-dnd", "rust/crates/db/src/models/notification_push.rs",
     "dnd_exception: false", "dnd_exception: true",
     "ws17_event_board_and_huddle_pushers_match_50_actual_rails_source_cases", "campfire_db"),
    ("push-event-stale", "rust/crates/db/src/models/notification_push.rs",
     "if event.stale(now)", "if false && event.stale(now)",
     "ws17_event_board_and_huddle_pushers_match_50_actual_rails_source_cases", "campfire_db"),
    ("push-huddle-boundary", "rust/crates/db/src/models/notification_push.rs",
     "last_huddle_join_push_at<?", "last_huddle_join_push_at<=?",
     "ws17_event_board_and_huddle_pushers_match_50_actual_rails_source_cases", "campfire_db"),
    ("push-huddle-atomic", "rust/crates/db/src/models/notification_push.rs",
     "tx.emit_after_commit(Event::job(&HuddleJoinDeliveryJob {", 'tx.conn().execute_batch("COMMIT")?; tx.emit_after_commit(Event::job(&HuddleJoinDeliveryJob {',
     "ws17_huddle_join_enqueue_failure_rolls_back_throttle_and_source_write", "campfire"),
    ("calendar-steady-write", "rust/crates/db/src/models/calendar_dispatch.rs",
     "cache.is_some_and(|cache| cache.in_meeting_broadcast != Some(active))", "cache.is_some()",
     "ws17_calendar_two_ticks_match_rails_jobs_claims_update_counts_and_emission_order", "campfire_db"),
    ("calendar-racing-claim", "rust/crates/db/src/models/user_status_settings.rs",
     "AND (in_meeting_broadcast IS NULL OR in_meeting_broadcast!=?)", "AND (? IS NOT NULL)",
     "ws17_meeting_cache_claims_flip_once_across_loaded_records_and_processes", "campfire_db"),
    ("calendar-duplicate-refresh", "rust/crates/db/src/models/calendar_dispatch.rs",
     "user.ooo_calendar_enabled && !user.meeting_status_enabled && stale", "user.ooo_calendar_enabled && stale",
     "ws17_both_optins_refresh_through_meeting_dispatcher_only", "campfire"),
    ("calendar-atomic", "rust/crates/db/src/models/calendar_dispatch.rs",
     "tx.emit_after_commit(Event::job(&MeetingRefreshJob { user_id: id }));", 'tx.conn().execute_batch("COMMIT")?; tx.emit_after_commit(Event::job(&MeetingRefreshJob { user_id: id }));',
     "ws17_meeting_failing_member_does_not_stop_sweep_and_rolls_back_claim", "campfire"),
    ("status-ooo-claim-guard", "rust/crates/db/src/models/user_status_settings/updates.rs",
     "AND (ooo_until IS NULL OR ooo_until<=?)", "AND (1 OR ooo_until<=?)",
     "ws17_manual_ooo_claims_match_actual_rails_conditional_updates", "campfire_db"),
    ("status-transaction", "rust/crates/db/src/models/user_status_settings/updates.rs",
     "self.save(tx)?;", 'self.save(tx)?; tx.conn().execute_batch("COMMIT")?;',
     "ws17_failed_status_refresh_enqueue_rolls_back_the_entire_http_write", "campfire"),
    ("status-stream-target", "rust/crates/campfire/src/channels/sink.rs",
     'Some("status_badge")', 'Some("wrong_badge")',
     "ws17_opting_out_while_in_a_meeting_broadcasts_the_cleared_badge", "campfire"),
    ("status-seeded-error", "rust/crates/campfire/src/controllers/users/profiles.rs",
     "if enabled {", "if false && enabled {",
     "ws17_seeded_enabled_2fa_settings_errors_match_the_actual_rails_failure", "campfire"),
]
cases += [
    ("dm-viewer-scope", "rust/crates/campfire/src/controllers/presenters/status_settings.rs",
     "users.id!=?", "users.id>=?",
     "ws17_dm_ooo_renders_per_viewer_without_shared_fragment", "campfire"),
    ("dm-streams-for-future-ooo", "rust/crates/campfire/src/controllers/presenters/status_settings.rs",
     "AND users.role!=? ORDER BY", "AND users.role!=? AND users.ooo_until IS NOT NULL ORDER BY",
     "ws17_dm_with_nobody_out_has_no_notice", "campfire"),
    ("dm-profile-live-presence", "rust/crates/campfire/src/controllers/presenters/status_settings.rs",
     "let presence = match presence {", "let presence = match campfire_db::models::workspace_presence_lease::Presence::Offline {",
     "ws17_profile_mounts_live_badge_and_viewer_scoped_allowance_control", "campfire"),
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
