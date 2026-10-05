#!/usr/bin/env python3
"""Reject representative producer defects using the same real-path receipt tests.

Temporarily edit actual Rust writers/templates, require assertion failures (not
compile/transport failures), and restore all inputs before returning.
"""
import os
import re
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[1]
scratch = root / 'target/controller-receipt-controls'
scratch.mkdir(parents=True, exist_ok=True)
controls = [
    ('profile-save', 'crates/db/src/models/user/profile_settings.rs',
     'crate::slash_commands::user_settings::update(tx, user, Value::Object(attrs))',
     'let _ = (tx, user, attrs); Ok(())',
     'controllers::users::cutover_receipts_tests::original_profile_text_size_save_matches_rails',
     'original_text_size'),
    ('profile-default', 'crates/campfire/src/controllers/presenters/profile_sections.rs',
     'enabled:!matches!(&inbox[key],serde_json::Value::Bool(false))',
     'enabled:false && !matches!(&inbox[key],serde_json::Value::Bool(false))',
     'controllers::users::cutover_receipts_tests::original_profile_defaults_and_zone_options_match_rails_through_http',
     'zone null'),
    ('card-call', 'crates/views/templates/users/cards/show.html',
     '{% if !person.user.bot() %}', '{% if person.user.bot() %}',
     'controllers::users::people_tests::agents_can_be_messaged_but_not_called',
     'complete routed turbo-frame'),
    ('public-escaping', 'crates/views/templates/public_pages/about.html',
     '{{ operator_name }}', '{{ operator_name|safe }}',
     'controllers::public_pages::tests::public_page_bodies_match_rails_with_operator_escaping_and_email_uri_encoding',
     'first differing byte'),
    ('tour-save', 'crates/db/src/models/user.rs',
     'SET tour_completed_at=?,updated_at=?', 'SET created_at=?,updated_at=?',
     'controllers::users::preferences_tests::tour_touch_matches_rails_and_refreshes_on_repeated_completion',
     'InvalidColumnType'),
    ('zone-write', 'crates/db/src/models/user.rs',
     'serde_json::json!({"time_zone":zone})', 'serde_json::json!({"time_zone":"UTC"})',
     'controllers::users::preferences_tests::time_zone_detection_matches_rails_validation_and_saved_choice_vectors', 'assertion'),
    ('tour-shell', 'crates/views/templates/layouts/_tour.html',
     '{{ ctx.tour_pending() }}', 'false',
     'controllers::users::preferences_tests::tour_stamp_controls_the_room_layout_auto_start', 'exact original tour shell selector'),
    ('dnd-presence', 'crates/campfire/src/controllers/presenters/layout_preferences.rs',
     '|| presence == "dnd"', '|| presence == "mutant"',
     'controllers::users::layout_preferences_tests::layout_dnd_presence_mutes_sounds', 'notification-dnd: exact meta selector'),
    ('quiet-window', 'crates/campfire/src/controllers/presenters/layout_preferences.rs',
     'quiet_hours: if quiet { start.zip(end) } else { None }', 'quiet_hours: if quiet { end.zip(start) } else { None }',
     'controllers::users::layout_preferences_tests::layout_quiet_hours_window_and_zone', 'quiet-hours: exact meta selector'),
    ('future-meeting', 'crates/campfire/src/controllers/presenters/layout_preferences.rs',
     'sounds.meeting_quiet = window_epochs(&busy);', 'sounds.meeting_quiet = window_epochs(&busy); sounds.meeting_quiet.clear();',
     'controllers::users::layout_preferences_tests::layout_future_meeting_windows_before_start', 'meeting-quiet: exact meta selector'),
    ('manual-ooo', 'crates/campfire/src/controllers/presenters/layout_preferences.rs',
     '.map(|until| vec![(0, until.as_second())])', '.map(|until| vec![(1, until.as_second())])',
     'controllers::users::layout_preferences_tests::layout_manual_ooo_windows', 'ooo-quiet: exact meta selector'),
    ('drive-meta', 'crates/views/templates/layouts/application.html',
     '{% if ctx.google_drive_previews() %}', '{% if !ctx.google_drive_previews() %}',
     'controllers::users::layout_preferences_tests::layout_drive_previews_uses_exact_scope', 'google-drive-previews: exact meta selector'),
    ('calendar-email', 'crates/views/templates/users/profiles/_google_calendar.html',
     'Connected as {{ account.email }}', 'Connected as mutant',
     'controllers::users::profile_sections_tests::configured_calendar_profile_uses_real_account_metadata_and_forms', 'calendar_only'),
    ('account-name', 'crates/campfire/src/controllers/accounts.rs',
     'account.update(tx, name.as_deref(), None, settings.as_deref())?', 'let _ = &name; account.update(tx, None, None, settings.as_deref())?',
     'controllers::accounts::mutation_tests::account_mutations_and_audits_match_pinned_rails_http_vectors', 'settings_name'),
    ('pwa-type', 'crates/campfire/src/controllers/pwa.rs',
     'StatusCode::OK, "text/javascript; charset=utf-8", pwa::SERVICE_WORKER_JS', 'StatusCode::OK, "text/plain; charset=utf-8", pwa::SERVICE_WORKER_JS',
     'controllers::pwa::tests::pwa_http_bodies_match_rails_before_and_after_first_run', 'assertion'),

]
# The same omitted writer/call controls must also reject these separately credited tests.
extra_tests = [
    ('profile-save-notifications', 'controllers::users::cutover_receipts_tests::original_profile_notification_switch_save_matches_rails', 'original_notifications'),
    ('card-call-peer', 'controllers::users::people_tests::card_shows_identity_presence_role_and_actions_for_a_peer', 'complete routed turbo-frame'),
    ('card-call-offline', 'controllers::users::people_tests::offline_peers_read_offline', 'complete routed turbo-frame'),
]
originals = {}
try:
    for name, relative, before, after, test, assertion in controls:
        path = root / relative
        source = path.read_text()
        assert source.count(before) == 1, (name, 'producer mutation anchor', source.count(before))
        originals.setdefault(path, path.read_bytes())
        path.write_text(source.replace(before, after))
    tests = [(r[0], r[4], r[5]) for r in controls] + extra_tests
    expression = ' or '.join(f'test(={test})' for _, test, _ in tests)
    env = os.environ.copy()
    env['CI'] = '1'
    env['RUST_TEST_THREADS'] = '4'
    env['CAMPFIRE_REFERENCE'] = str(root.parent)
    command = ['cargo', 'nextest', 'run', '--locked', '-p', 'campfire', '-j', '4', '--no-fail-fast', '-E', expression]
    run = subprocess.run(command, cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    (scratch / 'mutants.log').write_text(run.stdout)
    assert run.returncode == 100 and 'Summary' in run.stdout, run.stdout[-5000:]
    for name, test, assertion in tests:
        assert re.search(r'^\s*FAIL\s+\[[^\]]+\].*'+re.escape(test)+r'$', run.stdout, re.M), (name, 'test did not fail')
        # Native nextest must attribute a test assertion/panic to this exact identity.
        segment = run.stdout.split(f"thread '{test}'", 1)[-1].split("thread '", 1)[0]
        assert f"thread '{test}'" in run.stdout and assertion in segment, (name, 'invalid control', segment)
        print(f'Controller receipt control {name}: intended assertion rejected actual producer defect')
    print(f'Controller receipt discrimination: {len(controls)} actual producer defects rejected by {len(tests)} distinct tests; 0 invalid controls')
finally:
    for path, source in originals.items():
        path.write_bytes(source)
