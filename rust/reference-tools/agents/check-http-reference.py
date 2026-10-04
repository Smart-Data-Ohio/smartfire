#!/usr/bin/env python3
"""Check every Rails source used by these adapters against the pinned oracle."""

from pin_identity import PIN, PIN_FULL, PIN_IMAGE
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
    files += ["app/services/agents/rooms.rb", "app/services/agents/reading.rb", "app/services/agents/pins.rb", "app/services/agents/polls.rb", "app/services/agents/work_threads.rb", "app/services/agents/board_posts.rb", "app/models/message_pin.rb", "app/models/poll.rb", "app/models/poll_option.rb", "app/models/poll_vote.rb", "app/models/channel_thread.rb", "app/models/agents/work_payload.rb"]
    files += ["app/services/agents/reactions.rb", "app/models/boost.rb"]
    files += sorted(str(p.relative_to(root)) for p in (root / "app/controllers/agents").rglob("*.rb") if str(p.relative_to(root)) not in files)
    files += ["app/controllers/messages/by_bots_controller.rb", "app/controllers/messages/boosts/by_bots_controller.rb", "app/controllers/concerns/fizzy_agent_authentication.rb", "app/services/agents/fizzy_reads.rb", "app/services/agents/fizzy_card_actions.rb", "app/models/fizzy/agent_card_action.rb", "app/models/github/agent_pull_request_action.rb", "app/models/github/review_logins.rb", "app/models/github/agent_identity.rb", "app/models/github_connected_account.rb", "app/models/fizzy_connected_account.rb", "app/models/fizzy/client.rb", "app/views/users/_user.json.jbuilder", "app/views/messages/_message.json.jbuilder"]
    files += ["app/controllers/concerns/set_time_zone.rb", "app/models/message/attachment.rb", "app/models/message/pagination.rb", "test/fixtures/files/moon.jpg"]
    files += ["app/controllers/messages_controller.rb", "app/models/user/bot.rb", "app/models/message/bot_webhook_fanout.rb", "app/models/message/mention_preloader.rb", "app/models/agent/delivery.rb", "app/jobs/bot/webhook_job.rb"]
    files += ["app/models/work_thread_link.rb", "app/models/github/pull_request.rb"]
    files += ["app/services/agents/work_handoffs.rb", "app/models/work_handoff.rb", "app/models/work_thread_event.rb", "app/models/board_tag_assignment.rb"]
    files += ["app/models/activity_item.rb", "app/models/agent_budget_notice.rb", "app/services/agents/budgets.rb", "app/services/activity_items/recorder.rb", "app/models/webhook.rb"]
    assert (root / "test/fixtures/files/moon.jpg").read_bytes() == (root / "rust/vectors/users_logos/moon.jpg").read_bytes()
    lines = subprocess.check_output([
        "docker", "run", "--rm", "--name", "ws11api-source-check", "--entrypoint", "sha256sum",
        PIN_IMAGE, *["/rails/" + file for file in files],
    ], text=True).splitlines()
    for file, line in zip(files, lines, strict=True):
        pinned = subprocess.check_output(["git", "show", f"{PIN_FULL}:{file}"], cwd=root)
        check_source(pinned, line.split()[0], (root / file).read_bytes(), file)
    named_files=["test/models/message_streaming_test.rb", "test/models/user/bot_test.rb", "test/models/channel_thread_agent_assignment_test.rb", "test/models/agent_budgets_test.rb"]
    for file in named_files:
        assert (root/file).read_bytes()==subprocess.check_output(["git","show",f"{PIN_FULL}:{file}"],cwd=root),file
    print("WS11-api new named sources: 4 pinned test files matched checkout; test sources are not shipped in the Rails image")
    print(f"WS11-api reference sources: {len(files)} pinned files matched; 0 image or checkout mismatches ({PIN})")


if __name__ == '__main__':
    main()
