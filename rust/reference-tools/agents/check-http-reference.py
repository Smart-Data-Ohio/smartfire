#!/usr/bin/env python3
"""Check every Rails source used by these adapters against the pinned oracle."""
import hashlib
from pathlib import Path
import subprocess


def check_source(pinned, image_hash, checkout, file):
    assert hashlib.sha256(pinned).hexdigest() == image_hash, file
    assert checkout == pinned, file


def main():
    root = Path(__file__).resolve().parents[3]
    files = [
    "app/controllers/agents_controller.rb",
    "app/controllers/agents/events_controller.rb", "app/controllers/agents/steps_controller.rb",
    "app/controllers/agents/slash_commands_controller.rb", "app/controllers/agents/mcp_controller.rb",
    "app/controllers/concerns/authentication.rb", "app/controllers/concerns/agent_api_throttle.rb",
    "app/controllers/concerns/agent_authorization.rb", "app/controllers/application_controller.rb",
    "app/services/agents/mcp_server.rb", "app/services/agents/event_polling.rb",
    "app/services/agents/steps.rb", "app/services/agents/slash_commands.rb",
    "app/models/user.rb", "app/models/user/bannable.rb",
    "app/services/periodic/runner.rb", "app/services/agents/streaming.rb", "app/models/message.rb", "app/models/message/broadcasts.rb",
    "app/services/agents/context_builder.rb", "app/services/agents/posting.rb", "app/services/agents/direct_messages.rb",
    "app/services/agents/working_presence.rb", "app/services/agents/service_result.rb",
    "app/models/agent.rb", "app/models/agent_credential.rb", "app/models/agent_grant.rb",
    "app/models/agent_step.rb", "app/models/agent_slash_command.rb",
    "app/helpers/message_payload_helper.rb",
    ]
    files += sorted(str(p.relative_to(root)) for p in (root / "app/controllers/agents").rglob("*.rb") if str(p.relative_to(root)) not in files)
    files += ["app/controllers/messages/by_bots_controller.rb", "app/controllers/messages/boosts/by_bots_controller.rb", "app/controllers/concerns/fizzy_agent_authentication.rb", "app/services/agents/fizzy_reads.rb", "app/services/agents/fizzy_card_actions.rb", "app/models/fizzy/agent_card_action.rb", "app/models/github/agent_pull_request_action.rb", "app/models/github/review_logins.rb", "app/models/github/agent_identity.rb", "app/models/github_connected_account.rb", "app/models/fizzy_connected_account.rb", "app/models/fizzy/client.rb", "app/views/users/_user.json.jbuilder", "app/views/messages/_message.json.jbuilder"]
    lines = subprocess.check_output([
        "docker", "run", "--rm", "--name", "ws11api-source-check", "--entrypoint", "sha256sum",
        "ws11api-reference:d7c7de92", *["/rails/" + file for file in files],
    ], text=True).splitlines()
    for file, line in zip(files, lines, strict=True):
        pinned = subprocess.check_output(["git", "show", f"d7c7de92:{file}"], cwd=root)
        check_source(pinned, line.split()[0], (root / file).read_bytes(), file)
    print(f"WS11-api reference sources: {len(files)} pinned files matched; 0 image or checkout mismatches (d7c7de92)")


if __name__ == '__main__':
    main()
