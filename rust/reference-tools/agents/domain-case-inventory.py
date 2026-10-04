#!/usr/bin/env python3
"""Inventory pinned WS11 domain tests; source cases are never reported as executed tests."""

from pin_identity import PIN, PIN_FULL, PIN_IMAGE
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
EXPLICIT_FILES = {
    "test/models/channel_thread_agent_assignment_test.rb",
    "test/models/message/bot_webhook_fanout_test.rb",
    "test/models/message_streaming_test.rb",
    "test/models/user/bot_test.rb",
    "test/models/webhook_agent_key_test.rb",
    "test/models/webhook_test.rb",
    "test/lib/restricted_http/private_network_guard_test.rb",
    "test/services/bots/clear_plaintext_tokens_test.rb",
    "test/services/slash_commands/dispatcher_test.rb",
}
paths = subprocess.check_output(
    ["git", "ls-tree", "-r", "--name-only", PIN, "test"], cwd=ROOT, text=True
).splitlines()
selected = []
for path in paths:
    if not (
        path.startswith("test/models/agent")
        or path.startswith("test/jobs/agent/")
        or path in EXPLICIT_FILES
    ):
        continue
    source = subprocess.check_output(
        ["git", "show", f"{PIN}:{path}"], cwd=ROOT, text=True
    )
    cases = re.findall(r'^\s*test\s+["\'](.*?)["\']\s+do', source, re.M)
    selected.append({"path": path, "cases": len(cases), "names": cases})

output = ROOT / ".scratch" / "ws11-domain-case-inventory.json"
output.parent.mkdir(exist_ok=True)
output.write_text(json.dumps(selected, indent=2) + "\n")
print(
    f"WS11 domain inventory: {len(selected)} pinned Rails files; "
    f"{sum(row['cases'] for row in selected)} source cases; "
    "0 source cases claimed as run"
)
