#!/usr/bin/env python3
"""Check every Rails source used by these adapters against the pinned oracle."""
import hashlib
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[3]
files = [
    "app/controllers/agents_controller.rb",
    "app/controllers/agents/events_controller.rb", "app/controllers/agents/steps_controller.rb",
    "app/controllers/agents/slash_commands_controller.rb", "app/controllers/agents/mcp_controller.rb",
    "app/controllers/concerns/authentication.rb", "app/controllers/concerns/agent_api_throttle.rb",
    "app/controllers/concerns/agent_authorization.rb", "app/controllers/application_controller.rb",
    "app/services/agents/mcp_server.rb", "app/services/agents/event_polling.rb",
    "app/services/agents/steps.rb", "app/services/agents/slash_commands.rb",
    "app/services/agents/working_presence.rb", "app/services/agents/service_result.rb",
    "app/models/agent.rb", "app/models/agent_credential.rb", "app/models/agent_grant.rb",
    "app/models/agent_step.rb", "app/models/agent_slash_command.rb",
    "app/helpers/message_payload_helper.rb",
]
lines = subprocess.check_output([
    "docker", "run", "--rm", "--name", "ws11api-source-check", "--entrypoint", "sha256sum",
    "ws11api-reference:d7c7de92", *["/rails/" + file for file in files],
], text=True).splitlines()
for file, line in zip(files, lines, strict=True):
    pinned = subprocess.check_output(["git", "show", f"d7c7de92:{file}"], cwd=root)
    assert hashlib.sha256(pinned).hexdigest() == line.split()[0], file
    assert (root / file).read_bytes() == pinned, file
print(f"WS11-api reference sources: {len(files)} pinned files matched; 0 image or checkout mismatches (d7c7de92)")
