#!/usr/bin/env python3
"""Compare regenerated Rails outputs exactly and report the deliberately partial MCP coverage."""
from pathlib import Path
import json
import re
import sys

def compare_vectors(actual, expected, filename):
    assert actual == expected, filename


def check_dispatch(names, source):
    # A match arm can contain alternatives, e.g. register | unregister.
    branches = set(re.findall(r'"([a-z_]+)"\s*(?:\||=>)', source))
    assert names <= branches, names - branches


def main():
    root = Path(__file__).resolve().parents[2]
    scratch = Path(sys.argv[1]).resolve()
    for kind, filename in [('HTTP', 'agent_http.json'), ('MCP', 'agent_mcp.json'), ('surface', 'agent_surface.json'), ('bot', 'agent_bot_http.json'), ('conversation', 'agent_conversation_http.json'), ('Fizzy reads', 'agent_fizzy_http.json'), ('Fizzy approvals', 'agent_fizzy_action_http.json'), ('readers', 'agent_reads_http.json'), ('pins', 'agent_pins_http.json'), ('polls', 'agent_polls_http.json')]:
        expected = (root / 'vectors' / filename).read_bytes()
        actual = (scratch / filename).read_bytes()
        compare_vectors(actual, expected, filename)
        print(f'WS11-api fresh {kind} oracle: {len(json.loads(actual)["cases"])} request/response pairs; byte-identical committed vectors')
    metadata = json.loads((root / 'crates/campfire/src/controllers/agents/mcp_metadata.json').read_text())
    names = {tool['name'] for tool in metadata['tools']}
    check_dispatch(names, (root / 'crates/campfire/src/controllers/agents/mcp.rs').read_text())
    print(f'WS11-api MCP dispatch: {len(names)} explicit tool names; {sum(bool(t["throttle"]) for t in metadata["tools"])} throttled tools')
    cases = json.loads((root / 'vectors/agent_mcp.json').read_text())['cases']
    asserted = set(re.findall(r'"([^"]+)"', (root / 'crates/campfire/src/controllers/agent_mcp_tests.rs').read_text()))
    covered = sum(c['name'] in asserted for c in cases)
    print(f'WS11-api base MCP coverage: {covered} asserted vectors; {len(cases) - covered} deferred base vectors')
    surface = json.loads((root / 'vectors/agent_surface.json').read_text())['cases']
    assert sum(c['name'].startswith('matrix_rest_invalid_') for c in surface) == 35
    assert sum(c['name'].startswith('matrix_rest_session_') for c in surface) == 35
    assert sum(c['name'].startswith('mcp_matrix_rate_') for c in surface) == 29
    print('WS11-api boundary matrix: 35 REST invalid credentials; 35 REST human sessions; 29 MCP tool overflows')


if __name__ == '__main__':
    main()
