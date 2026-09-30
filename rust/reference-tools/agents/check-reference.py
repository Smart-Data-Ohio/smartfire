#!/usr/bin/env python3
"""Verify the Rails sources used by this slice against its pinned image."""
import hashlib
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[3]
files = [
    'app/models/user/bot.rb', 'app/models/agent.rb', 'app/models/agent_credential.rb', 'app/models/agent_grant.rb',
    'app/controllers/concerns/authentication.rb', 'app/controllers/concerns/agent_authorization.rb',
    'app/controllers/accounts/bots/keys_controller.rb', 'app/controllers/messages/by_bots_controller.rb',
    'app/controllers/messages/boosts/by_bots_controller.rb', 'app/views/accounts/bots/keys/show.html.erb',
    'app/services/bots/clear_plaintext_tokens.rb',
    'Gemfile.lock', 'app/models/webhook.rb', 'app/models/message/bot_webhook_fanout.rb',
    'app/models/agent/delivery.rb', 'app/models/agent_event.rb', 'app/models/drive_attachment.rb',
    'lib/restricted_http/private_network_guard.rb', 'app/jobs/bot/webhook_job.rb', 'app/jobs/application_job.rb',
]
command = ['docker', 'run', '--rm', '--name', 'ws11-reference-source-check', '--entrypoint', 'sha256sum',
           'triage-reference-d7c7de92', *['/rails/' + file for file in files]]
lines = subprocess.check_output(command, text=True).splitlines()
for file, line in zip(files, lines, strict=True):
    assert hashlib.sha256((root / file).read_bytes()).hexdigest() == line.split()[0], file
print(f'WS11 reference sources: {len(files)} matched; 0 mismatched (d7c7de92)')
