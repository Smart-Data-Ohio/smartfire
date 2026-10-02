#!/usr/bin/env python3
"""Compare regenerated Rails outputs exactly and report the deliberately partial MCP coverage."""
from pathlib import Path
import json
import re
import sys

def compare_vectors(actual, expected, filename):
    assert actual == expected, filename


def check_approved_difference(vector):
    assert len(vector['cases']) == 1
    case = vector['cases'][0]
    assert case['name'] == 'fresh_jpeg_closed_thread'
    assert case['rails']['status'] == 500
    assert case['approved']['status'] == 201
    assert case['exception']['class'] == 'IOError'
    assert case['exception']['message'] == 'closed stream'
    assert 'disk_service.rb:23' in case['exception']['backtrace'][0]
    assert case['state']['variant_file_exists'] is False
    assert case['state']['variant_file_size'] is None
    assert case['state']['thread']['closed_at'] is None
    assert [job['class'] for job in case['state']['jobs']] == ['ChannelThread::PushMessageJob']
    assert json.loads(case['approved']['response'])['id'] == case['state']['message']['id']


def check_approved_video_difference(vector):
    assert len(vector['cases']) == 1
    case = vector['cases'][0]
    assert case['name'] == 'fresh_video_closed_thread'
    assert case['rails']['status'] == 500
    assert case['approved']['status'] == 201
    assert case['exception']['class'] == 'ActiveStorage::FileNotFoundError'
    assert 'disk_service.rb:152' in case['exception']['backtrace'][0]
    assert case['exception']['open_transactions'] == 1
    assert all(delta == 0 for delta in case['rails_state']['row_deltas'].values())
    assert case['rails_state']['thread_closed_at'] == '2026-03-01 16:00:00 UTC'
    assert case['rails_state']['source_file_exists'] is True
    assert case['rails_state']['jobs'] == []
    assert case['state']['thread']['closed_at'] is None
    assert case['state']['variant_count'] == 1
    assert len(case['state']['blobs']) == 3
    for blob in case['state']['blobs']:
        assert blob['file_exists'] is True
        assert blob['file_size'] == blob['attributes']['byte_size'] > 0
    assert [job['class'] for job in case['state']['jobs']] == ['ActiveStorage::AnalyzeJob', 'ChannelThread::PushMessageJob']
    assert case['state']['jobs'][0]['args']['blob_id'] == case['state']['blobs'][2]['attributes']['id']
    assert json.loads(case['approved']['response'])['id'] == case['state']['message']['id']
    attachments = case['state']['attachments']
    assert len(attachments) == 3
    source, preview, image = [blob['attributes']['id'] for blob in case['state']['blobs']]
    assert [(a['record_type'], a['record_id'], a['name'], a['blob_id']) for a in attachments] == [
        ('ActiveStorage::Blob', source, 'preview_image', preview),
        ('ActiveStorage::VariantRecord', 7, 'image', image),
        ('Message', case['state']['message']['id'], 'attachment', source),
    ]


def check_dispatch(names, source):
    # A match arm can contain alternatives, e.g. register | unregister.
    branches = set(re.findall(r'"([a-z_]+)"\s*(?:\||=>)', source))
    assert names <= branches, names - branches


def main():
    root = Path(__file__).resolve().parents[2]
    scratch = Path(sys.argv[1]).resolve()
    for kind, filename in [('HTTP', 'agent_http.json'), ('MCP', 'agent_mcp.json'), ('surface', 'agent_surface.json'), ('bot', 'agent_bot_http.json'), ('legacy bot/fanout/replacement', 'agent_legacy_bot_http.json'), ('conversation', 'agent_conversation_http.json'), ('Fizzy reads', 'agent_fizzy_http.json'), ('Fizzy approvals', 'agent_fizzy_action_http.json'), ('readers', 'agent_reads_http.json'), ('pins', 'agent_pins_http.json'), ('polls', 'agent_polls_http.json'), ('polling', 'agent_polling_http.json'), ('reactions', 'agent_reactions_http.json'), ('bot reactions', 'agent_bot_reactions_http.json'), ('work validation', 'agent_work_validation_http.json'), ('attachments', 'agent_attachments_http.json'), ('permissions', 'agent_permissions_http.json'), ('PR192 zones and ID shapes', 'agent_review192_http.json'), ('PR192 approved JPEG difference', 'agent_review192r2_attachment.json'), ('PR192 approved video difference', 'agent_review192r3_attachment.json'), ('PR192 handled missing representations', 'agent_review192r5_representations.json')]:
        expected = (root / 'vectors' / filename).read_bytes()
        actual = (scratch / filename).read_bytes()
        compare_vectors(actual, expected, filename)
        if filename == 'agent_review192r2_attachment.json':
            check_approved_difference(json.loads(actual))
        if filename == 'agent_review192r3_attachment.json':
            check_approved_video_difference(json.loads(actual))
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
