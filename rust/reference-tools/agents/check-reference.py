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
    'app/jobs/agent/delivery_job.rb', 'app/jobs/agent/event_webhook_job.rb',
    'app/models/message/agent_delivery.rb', 'app/models/agents/work_payload.rb', 'app/models/work_thread_link.rb',
    'app/models/github/pull_request.rb', 'app/models/github/pull_request_thread.rb', 'app/models/github_connected_account.rb',
    'app/models/membership.rb', 'app/models/user.rb', 'app/models/boost.rb',
    'app/services/agents/event_polling.rb', 'app/services/agents/service_result.rb',
    'app/services/agents/posting.rb', 'app/services/agents/budgets.rb', 'app/models/agent_budget_notice.rb',
    'app/models/agent_approval.rb', 'app/services/agents/approvals.rb',
    'app/models/agent_step.rb', 'app/services/agents/steps.rb',
    'app/models/agent_slash_command.rb', 'app/services/agents/slash_commands.rb',
    'app/services/slash_commands/dispatcher.rb', 'app/models/activity_item.rb',
    'app/services/agents/working_presence.rb', 'app/models/channel_thread.rb',
]
command = ['docker', 'run', '--rm', '--name', 'ws11-reference-source-check', '--entrypoint', 'sha256sum',
           'triage-reference-d7c7de92', *['/rails/' + file for file in files]]
lines = subprocess.check_output(command, text=True).splitlines()
drift=[]
for file, line in zip(files, lines, strict=True):
    pinned=subprocess.check_output(['git','show',f'd7c7de92:{file}'],cwd=root)
    assert hashlib.sha256(pinned).hexdigest()==line.split()[0], file
    current=(root/file).read_bytes()
    if current!=pinned:
        # Authorized main merge includes #159's unrelated rubyzip security update. Keep
        # checking the actual pinned lock used by the oracle; allow exactly that checkout diff.
        assert file=='Gemfile.lock' and current==pinned.replace(b'rubyzip (3.0.2)',b'rubyzip (3.7.0)'), file
        drift.append(file)
print(f'WS11 reference sources: {len(files)} pinned files matched; 0 image mismatches (d7c7de92)')
if drift:print('WS11 checkout drift: Gemfile.lock rubyzip 3.0.2 -> 3.7.0 from merged main; oracle stays pinned')
