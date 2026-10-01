#!/usr/bin/env python3
"""Prove the new HTTP checks reject compiled regressions; always restore the source."""
from pathlib import Path
import os
import subprocess
import sys

root = Path(__file__).resolve().parents[2]
scratch = root.parent / ".scratch" / "users-discrimination"
scratch.mkdir(parents=True, exist_ok=True)
env = os.environ.copy()
env.update(CI="1", TMPDIR=str(root.parent / ".scratch" / "tmp"))
mutations = [
    ("agent-private-room", "crates/campfire/src/controllers/presenters/agent_profile.rs", "campfire_db::Membership::find_by_room_and_user(conn, room.id, viewer.id)?.is_some()", "true", "private_rooms_hidden"),
    ("agent-owner-actions", "crates/campfire/src/controllers/presenters/agent_profile.rs", "viewer.is_administrator()", "true", "peer_hides_grants"),
    ("signed-blob-filename", "crates/storage/src/blob.rs", "Some(&filename.sanitized())", "Some(filename.raw())", "signed_icon_and_logo_http_assignments_match_rails_filenames_metadata_and_jobs"),
    ("attachment-durable-enqueue", "crates/campfire/src/controllers/presenters/attachments.rs", "tx.emit_after_commit(Event::job(&AnalyzeJob { blob_id: blob.id }));", "let _ = blob.id;", "signed_icon_analysis_enqueue_failure_rolls_back_icon_identification_and_audit"),
    ("sign-in-display-route", "crates/views/templates/sessions/_google_sign_in.html", "h::routes::session_google().as_str()", "h::routes::session().as_str()", "configured_sign_in_keeps_public_links_and_matches_complete_rails_bodies"),
    ("sign-in-display-token", "crates/views/templates/sessions/_google_sign_in.html", 'h::attrs().method("post").class("btn center")', 'h::attrs().method("post").class("btn center").attr("authenticity_token", false)', "configured_sign_in_keeps_public_links_and_matches_complete_rails_bodies"),
    ("sign-in-display-credentials", "crates/campfire/src/config.rs", "if google_client_configured {", "if true {", "configured_sign_in_keeps_public_links_and_matches_complete_rails_bodies"),
    ("ooo-start-boundary", "crates/campfire/src/controllers/presenters/layout_preferences.rs", "start <= now && now < end", "start < now && now < end", "calendar_start_boundary"),
    ("ooo-end-boundary", "crates/campfire/src/controllers/presenters/layout_preferences.rs", "start <= now && now < end", "start <= now && now <= end", "calendar_end_boundary"),
    ("ooo-overlap-end", "crates/campfire/src/controllers/presenters/profile_sections.rs", "manual_end.into_iter().chain(calendar_end).max()", "manual_end.into_iter().chain(calendar_end).min()", "calendar_later_end"),
    ("layout-future-window", "crates/campfire/src/controllers/presenters/layout_preferences.rs", '.map(|(start, end)| (start.as_second(), end.as_second()))', '.filter(|(start, _)| start.as_second() <= 1772467200).map(|(start, end)| (start.as_second(), end.as_second()))', "layout_future_meeting_windows_before_start"),
    ("layout-meeting-gate", "crates/campfire/src/controllers/presenters/layout_preferences.rs", "row.get::<_, bool>(6)? && row.get::<_, bool>(7)?", "true", "layout_meeting_status_off_sends_no_windows"),
    ("layout-ooo-keep", "crates/campfire/src/controllers/presenters/layout_preferences.rs", "let keep_notifications: bool = row.get(10)?;", "let keep_notifications = false;", "layout_ooo_notifications_kept_sends_no_windows"),
    ("layout-drive-scope", "crates/campfire/src/controllers/presenters/layout_preferences.rs", 'scope == "https://www.googleapis.com/auth/drive.file"', 'scope == "https://www.googleapis.com/auth/drive.metadata.readonly"', "layout_drive_previews_uses_exact_scope"),
    ("avatar-webp-size", "crates/campfire/src/controllers/users/avatars.rs", 'Variation::resize_to_limit(512, 512, Some("webp"))', 'Variation::resize_to_limit(256, 256, Some("webp"))', "uploaded_avatar_image_uses_rails_bytes_headers_and_freshness"),
    ("avatar-bmp-fallback", "crates/campfire/src/controllers/users/avatars.rs", "else if user.is_bot() {", "else if !user.is_bot() {", "unresizable_avatar_falls_back_to_rails_initials_bytes_and_headers"),
    ("sign-in-public-link", "crates/views/templates/sessions/new.html", 'h::link_to_text("Privacy Policy", &h::routes::privacy()', 'h::link_to_text("Privacy Policy", &h::routes::about()', "unconfigured_sign_in_links_all_public_pages_in_new_tabs"),
    ("ban-setup-cleanup", "crates/db/src/models/user.rs", 'r#"DELETE FROM "sessions" WHERE "sessions"."user_id" = ?"#,\n            [self.id],\n        )?;\n        tx.emit_after_commit(Event::RemoveBannedContent', 'r#"DELETE FROM "sessions" WHERE "sessions"."user_id" = ? AND id < 0"#,\n            [self.id],\n        )?;\n        tx.emit_after_commit(Event::RemoveBannedContent', "banning_a_user_removes_pending_two_factor_setup_and_sessions"),
    ("ban-job-enqueue", "crates/db/src/models/user.rs", 'tx.emit_after_commit(Event::RemoveBannedContent { user_id: self.id });', '// deliberate dropped ban enqueue', "ban_http_enqueue_is_atomic_and_writes_one_durable_remove_job"),
    ("ban-job-perform", "crates/campfire/src/jobs.rs", 'for message in messages {\n        let (removed, room_id)', 'for message in messages.into_iter().take(0) {\n        let (removed, room_id)', "ban_http_removes_the_users_messages_through_the_real_runner"),
    ("dm-picker-view", "crates/views/templates/rooms/directs/new.html", "Type names to filter…", "Type names to filtez…", "seed_picker"),
    ("status-prefix", "crates/views/src/users/status_popup.rs", 'id_prefix: "status_popup".into()', 'id_prefix: "user".into()', "complete_popup_bodies_match_post_pin_rails"),
    ("status-redirect", "crates/campfire/src/controllers/users/statuses.rs", 'status: Some(StatusCode::SEE_OTHER)', 'status: Some(StatusCode::FOUND)', "popup_update_matches_rails_redirects_errors_and_current_user_state"),
    ("status-csrf", "crates/campfire/src/controllers/users/statuses.rs", 'pub async fn update(c: &mut Ctx) -> Result {\n    concerns::before_actions(c, Before::default()).await?;', 'pub async fn update(c: &mut Ctx) -> Result {\n    concerns::before_actions(c, Before::default().skip_forgery_protection()).await?;', "status_mutations_require_csrf_and_roll_back_on_write_failure"),
    ("status-scope", "crates/campfire/src/controllers/users/statuses.rs", 'pub async fn update(c: &mut Ctx) -> Result {\n    concerns::before_actions(c, Before::default()).await?;\n    let id = concerns::require_current_user(c)?.id;', 'pub async fn update(c: &mut Ctx) -> Result {\n    concerns::before_actions(c, Before::default()).await?;\n    let id = c.param_str("user_id").and_then(|value| value.parse().ok()).unwrap_or(concerns::require_current_user(c)?.id);', "popup_update_matches_rails_redirects_errors_and_current_user_state"),
    ("profile-calendar", "crates/campfire/src/controllers/users/profiles.rs", 'sections.google.calendar_configured = c.app().config.profile_google_calendar_configured;', 'sections.google.calendar_configured = false;', "configured_calendar_profile_uses_real_account_metadata_and_forms"),
    ("join-code", "crates/campfire/src/controllers/users.rs", 'c.param_str("join_code") != Some(account.join_code.as_str())', 'c.param_str("join_code") == Some(account.join_code.as_str())', "join_page_matches_complete_rails_body_and_access_checks"),
    ("first-run-repeat", "crates/campfire/src/controllers/first_runs.rs", "    if any {", "    if !any {", "signup_body_matches_rails_and_is_available_until_account_exists"),
    ("profile-inbox", "crates/views/templates/users/profiles/_inbox_calls.html", 'legend class="txt-large">Notifications', 'legend class="txt-large">Notificationz', "whole_profile_matches_rails_seed_without_masks"),
    ("icons-svg", "crates/storage/src/workspace_icon.rs", '"script" => return Some("must not contain script elements")', '"script_never" => return Some("must not contain script elements")', "all_committed_media_and_field_validation_cases_match_rails"),
    ("audit-csv-formula", "crates/campfire/src/controllers/accounts/audit_logs.rs", "['=', '+', '-', '@', '\\t', '\\r']", "['X', '+', '-', '@', '\\t', '\\r']", "html_navigation_csv_and_filtered_order_match_rails"),
    ("icons-header", "crates/campfire/src/controllers/workspace_icons.rs", 'c.set_header("x-content-type-options", "nosniff");', 'c.set_header("x-content-type-options", "sniff");', "serving_svg_and_png_uses_private_bytes_and_checksum_conditionals"),
    ("account-view-auth", "crates/views/templates/accounts/users/_user.html", "{% if ctx.can_administer() && user.active() %}", "{% if !ctx.can_administer() && user.active() %}", "account_member_rows_match_google_security_role_and_inactive_rails_controls"),
    ("account-view-google", "crates/views/src/users/summary.rs", "self.email_self_changed || !self.google_email_link_allowed", "self.email_self_changed || self.google_email_link_allowed", "account_member_rows_match_google_security_role_and_inactive_rails_controls"),
    ("audit-duplicate", "crates/campfire/src/account_security.rs", "    )?;\n    Ok(())\n}\n\npub fn settings_changed", "    )?;\n    AuditLog::record(tx, NewAuditLog { action: action.into(), target: Some(Target::from(account)), ..Default::default() }, context)?;\n    Ok(())\n}\n\npub fn settings_changed", "settings_name_writes_exactly_one_rails_audit_row"),
    ("account-admin", "crates/campfire/src/controllers/accounts.rs", "concerns::ensure_can_administer(c)?;", "let _ = concerns::require_current_user(c)?;", "account_and_ban_mutations_authorize_before_writes_and_audits"),
    ("account-sudo", "crates/campfire/src/controllers/accounts/join_codes.rs", "concerns::sudo::require_sudo_mode(c)?;", "// removed sudo for the discrimination check", "account_and_ban_mutations_authorize_before_writes_and_audits"),
    ("account-role", "crates/campfire/src/authentication.rs", "user.role != before", "user.role == before", "account_mutations_and_audits_match_pinned_rails_http_vectors"),
    ("ban-invalid", "crates/campfire/src/controllers/users/bans.rs", "Error::Status(StatusCode::UNPROCESSABLE_ENTITY)", 'Error::internal(anyhow::anyhow!("forced invalid record status"))', "account_mutations_and_audits_match_pinned_rails_http_vectors"),
    ("profile-settings", "crates/db/src/models/user/profile_settings.rs", "crate::slash_commands::user_settings::update(tx, user, Value::Object(attrs))", "let _ = attrs; Ok(())", "manual_profile_settings_match_pinned_rails_patch_vectors"),
    ("profile-fields", "crates/views/src/users/appearance.rs", "errors.is_empty()", "!errors.is_empty()", "appearance_partial_matches_all_pinned_rails_bytes"),
    ("people-auth", "crates/campfire/src/controllers/users/cards.rs", "Before::default()", "Before::default().allow_unauthenticated_access()", "cards_require_sign_in_and_unknown_people_are_not_found"),
    ("people-self", "crates/db/src/models/user/presentation.rs", "u.id!=:viewer", "u.id=:viewer", "directory_requires_sign_in_and_excludes_the_viewer"),
    ("people-star", "crates/db/src/models/user/presentation.rs", "!person.starred", "person.starred", "directories_match_complete_rails_body_and_starred_order"),
    ("people-call", "crates/views/templates/users/cards/show.html", "{% if !person.user.bot() %}", "{% if person.user.bot() %}", "agents_can_be_messaged_but_not_called"),
    ("people-status", "crates/db/src/models/user/presentation.rs", "expiry > now", "expiry < now", "card_shows_presence_dot_and_custom_status_badge"),
    ("public-date", "crates/campfire/src/public_policy.rs", r"\p{Decimal_Number}", r"\p{Number}", "policy_matches_rails_environment_vectors"),
    ("public-cookie", "crates/campfire/src/controllers/public_pages.rs", "let formats = c.formats()?;", "let _ = c.form_authenticity_token();\n    let formats = c.formats()?;", "public_pages_bypass_authentication_browser_and_private_state"),
    ("public-escaping", "crates/views/templates/public_pages/about.html", "{{ operator_name }}", "{{ operator_name|safe }}", "public_page_bodies_match_rails_with_operator_escaping_and_email_uri_encoding"),
    ("public-bytes", "crates/views/templates/public_pages/about.html", "<h1>About Smartfire</h1>", "<h1>About  Smartfire</h1>", "public_page_bodies_match_rails_with_operator_escaping_and_email_uri_encoding"),
    ("avatar-boundary", "crates/db/src/models/user.rs", r"\p{Mark}", "", "default_initials_svg_matches_rails_bytes_and_cache_validation"),
    ("qr-capacity", "crates/campfire/src/controllers/qr_code.rs", 'ok_or_else(|| Error::internal(anyhow::anyhow!("Data length exceed maximum capacity of version 40")))?', "ok_or(Error::Status(StatusCode::UNPROCESSABLE_ENTITY))?", "qr_http_matches_rails_bytes_cache_and_capacity_errors"),
    ("pwa-bytes", "crates/views/templates/pwa/service_worker.js", 'const STATIC_CACHE = "smartfire-static-v1"', 'const STATIC_CACHE = "smartfire-static-v2"', "pwa_http_bodies_match_rails_before_and_after_first_run"),
    ("preference-csrf", "crates/campfire/src/controllers/users/time_zones.rs", "Before::default()", "Before::default().skip_forgery_protection()", "preference_writes_require_session_and_csrf_and_scope_to_current_user"),
    ("preference-scope", "crates/campfire/src/controllers/users/time_zones.rs", "let user_id = concerns::require_current_user(c)?.id;", 'let user_id = c.param_str("user_id").and_then(|value| value.parse().ok()).unwrap_or(concerns::require_current_user(c)?.id);', "preference_writes_require_session_and_csrf_and_scope_to_current_user"),
    ("preference-explicit", "crates/db/src/models/user.rs", "&& !explicit &&", "&& (explicit || !explicit) &&", "time_zone_detection_matches_rails_validation_and_saved_choice_vectors"),
    ("preference-zone-case", "crates/db/src/slash_commands/time_parser.rs", "if !names.identifiers.contains(identifier)", "if names.identifiers.contains(identifier)", "time_zone_detection_matches_rails_validation_and_saved_choice_vectors"),
    ("tour-stamp", "crates/db/src/models/user.rs", "SET tour_completed_at=?,updated_at=?", "SET created_at=?,updated_at=?", "tour_touch_matches_rails_and_refreshes_on_repeated_completion"),
]
selected = set(sys.argv[1:])
count = 0
for name, relative, before, after, test in mutations:
    if selected and name not in selected:
        continue
    path = root / relative
    original = path.read_bytes()
    source = original.decode()
    assert source.count(before) == 1, (name, "mutation anchor", source.count(before))
    try:
        path.write_text(source.replace(before, after))
        run = subprocess.run(["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-p", "campfire", test, "--", "--nocapture", "--test-threads=4"], cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        (scratch / f"{name}.log").write_text(run.stdout)
        summaries = [line for line in run.stdout.splitlines() if line.startswith("test result:")]
        assert run.returncode == 101 and summaries and "FAILED" in summaries[-1] and f"::{test} ... FAILED" in run.stdout, (name, run.stdout[-4000:])
        print(f"{name}: {summaries[-1]}", flush=True)
        count += 1
    finally:
        path.write_bytes(original)
assert count, "no mutations selected"
print(f"WS8br2 discrimination: {count} compiled regressions detected; sources restored")
