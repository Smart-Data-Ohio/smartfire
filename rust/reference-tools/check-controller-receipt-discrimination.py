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
]
originals = {}
try:
    for name, relative, before, after, test, assertion in controls:
        path = root / relative
        source = path.read_text()
        assert source.count(before) == 1, (name, 'producer mutation anchor', source.count(before))
        originals[path] = path.read_bytes()
        path.write_text(source.replace(before, after))
    expression = ' or '.join(f'test(={row[4]})' for row in controls)
    env = os.environ.copy()
    env['CI'] = '1'
    env['RUST_TEST_THREADS'] = '4'
    env['CAMPFIRE_REFERENCE'] = str(root.parent)
    command = ['cargo', 'nextest', 'run', '--locked', '-p', 'campfire', '-j', '4', '--no-fail-fast', '-E', expression]
    run = subprocess.run(command, cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    (scratch / 'mutants.log').write_text(run.stdout)
    assert run.returncode == 100 and 'Summary' in run.stdout, run.stdout[-5000:]
    for name, _, _, _, test, assertion in controls:
        assert re.search(r'^\s*FAIL\s+\[[^\]]+\].*'+re.escape(test)+r'$', run.stdout, re.M), (name, 'test did not fail')
        # Native nextest must attribute a test assertion/panic to this exact identity.
        segment = run.stdout.split(f"thread '{test}'", 1)[-1].split("thread '", 1)[0]
        assert f"thread '{test}'" in run.stdout and assertion in segment, (name, 'invalid control', segment)
        print(f'Controller receipt control {name}: intended assertion rejected actual producer defect')
    print(f'Controller receipt discrimination: {len(controls)} actual producer defects rejected; 0 invalid controls')
finally:
    for path, source in originals.items():
        path.write_bytes(source)
