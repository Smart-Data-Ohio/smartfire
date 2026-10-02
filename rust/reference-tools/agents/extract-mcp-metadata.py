#!/usr/bin/env python3
"""Vendor the Rails MCP descriptions/schemas without a build-time Rails dependency."""
import json
from pathlib import Path

root = Path(__file__).resolve().parents[2]
vectors = json.loads((root / "vectors/agent_mcp.json").read_text())
metadata = {key: vectors[key] for key in ("versions", "server_name", "server_version", "instructions", "tools")}
target = root / "crates/campfire/src/controllers/agents/mcp_metadata.json"
target.parent.mkdir(parents=True, exist_ok=True)
target.write_text(json.dumps(metadata, ensure_ascii=False, indent=2) + "\n")
print(f"WS11-api MCP metadata: {len(metadata['tools'])} Rails tools; {len(metadata['versions'])} protocol versions")
