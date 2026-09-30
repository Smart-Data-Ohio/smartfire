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
        run = subprocess.run(["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j", "4", "-p", "campfire", test, "--", "--nocapture"], cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        (scratch / f"{name}.log").write_text(run.stdout)
        summaries = [line for line in run.stdout.splitlines() if line.startswith("test result:")]
        assert run.returncode == 101 and summaries and "FAILED" in summaries[-1] and f"::{test} ... FAILED" in run.stdout, (name, run.stdout[-4000:])
        print(f"{name}: {summaries[-1]}", flush=True)
        count += 1
    finally:
        path.write_bytes(original)
assert count, "no mutations selected"
print(f"WS8br2 discrimination: {count} compiled regressions detected; sources restored")
