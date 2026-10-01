#!/usr/bin/env python3
"""Prove the owned GitHub and Drive HTTP regressions reject broken production paths."""
from pathlib import Path
import os
import subprocess
ROOT = Path(__file__).resolve().parents[3]
SCRATCH = ROOT / '.scratch/owned-mutations'
SCRATCH.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, CI='1', CARGO_BUILD_JOBS='2', RUST_TEST_THREADS='8',
           CABLE_TEST_PORT_RANGE='52000-52049', MAIL_TEST_PORT_RANGE='52000-52049')
mutations = [
    ('cached-token-leak', 'rust/crates/campfire/src/controllers/presenters.rs',
     '.map(std::sync::Arc::new).map_err',
     '.map(|mut html| { html.push_str(r#"<input type="hidden" name="authenticity_token" value="foreign-session" />"#); std::sync::Arc::new(html) }).map_err',
     'cached_pages_refreshes_and_thread_pages'),
    ('csrf-check-bypassed', 'rust/crates/kit/src/ctx.rs',
     'if valid_origin && self.any_authenticity_token_valid() {',
     'if valid_origin && { let _ = self.any_authenticity_token_valid(); true } {',
     'cached_owned_forms_submit'),
    ('github-card-omitted', 'rust/crates/campfire/src/controllers/presenters.rs',
     'let github_cards_html = github::message_cards(self.conn, self.app, message)?;',
     'let github_cards_html = format!("<div id=\\\"github_pr_cards_message_{}\\\" class=\\\"github-pr-cards\\\"></div>\\n", message.client_message_id);',
     'complete_github_containers'),
    ('drive-author-bypassed', 'rust/crates/campfire/src/controllers/messages.rs',
     'if message.system_note || require_current_user(c)?.id != message.creator_id {',
     'if message.system_note || { let _ = require_current_user(c)?; false } {',
     'root_and_thread_drive_requests'),
]
for name, file, before, after, test in mutations:
    path = ROOT / file
    source = path.read_text()
    assert source.count(before) == 1, name
    try:
        path.write_text(source.replace(before, after))
        run = subprocess.run(['mise', 'exec', 'rust@1.98.1', '--', 'cargo', 'test', '--locked', '-j2',
                              '--manifest-path', 'rust/Cargo.toml', '-p', 'campfire', test, '--', '--nocapture'],
                             cwd=ROOT, env=env, capture_output=True, text=True)
        output = run.stdout + run.stderr
        (SCRATCH / f'{name}.log').write_text(output)
        summary = [line for line in output.splitlines() if line.startswith('test result:')]
        assert run.returncode and summary and '1 failed;' in summary[-1], output
        print(f'{name}: {summary[-1]}', flush=True)
    finally:
        path.write_text(source)
print('WS8bm owned mutations: 4 rejected; 0 survived; production files restored', flush=True)
