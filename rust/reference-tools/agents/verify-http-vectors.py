#!/usr/bin/env python3
"""Compare regenerated Rails outputs exactly and report the deliberately partial MCP coverage."""
from pathlib import Path
import json
import re
import sys

def compare_vectors(actual, expected, filename):
    assert actual == expected, filename


def check_committed_attachment(case):
    assert case['rails']['status'] == 201
    assert case['approved']['status'] == 201
    assert 'exception' not in case
    message = case['state']['message']
    assert json.loads(case['rails']['response'])['id'] == message['id']
    assert json.loads(case['approved']['response'])['id'] == message['id']
    assert case['state']['thread']['closed_at'] is None
    committed = case['rails_state']
    assert committed['open_transactions'] == 0
    assert committed['row_deltas'] == dict(messages=1, channel_threads=0, active_storage_blobs=0, active_storage_attachments=1, active_storage_variant_records=0)
    assert committed['thread_closed_at'] is None
    assert committed['source_file_exists'] is True
    assert committed['jobs'].count('Message::AttachmentProcessingJob') == 1
    assert committed['jobs'].count('ChannelThread::PushMessageJob') == 1
    assert all(job['class'] != 'Message::AttachmentProcessingJob' for job in case['state']['jobs'])


def check_attachment_success(vector):
    assert len(vector['cases']) == 1
    case = vector['cases'][0]
    assert case['name'] == 'fresh_jpeg_closed_thread'
    check_committed_attachment(case)
    assert case['state']['variant_file_exists'] is True
    assert case['state']['variant_file_size'] == case['state']['image']['byte_size'] > 0
    assert case['state']['attachment_count'] == 2
    assert [job['class'] for job in case['state']['jobs']] == ['ChannelThread::PushMessageJob', 'ActiveStorage::AnalyzeJob']
    assert case['state']['jobs'][1]['args']['blob_id'] == case['state']['image']['id']


def check_video_attachment_success(vector):
    assert len(vector['cases']) == 1
    case = vector['cases'][0]
    assert case['name'] == 'fresh_video_closed_thread'
    check_committed_attachment(case)
    assert case['state']['variant_count'] == 1
    blobs = case['state']['blobs']
    assert len(blobs) == 3
    for blob in blobs:
        assert blob['file_exists'] is True
        assert blob['file_size'] == blob['attributes']['byte_size'] > 0
    assert [job['class'] for job in case['state']['jobs']] == ['ChannelThread::PushMessageJob', *['ActiveStorage::AnalyzeJob'] * 3]
    assert [job['args']['blob_id'] for job in case['state']['jobs'][1:]] == [blob['attributes']['id'] for blob in blobs]
    attachments = case['state']['attachments']
    assert len(attachments) == 3
    source, preview, image = [blob['attributes']['id'] for blob in blobs]
    assert [(a['record_type'], a['name'], a['blob_id']) for a in attachments] == [
        ('Message', 'attachment', source), ('ActiveStorage::Blob', 'preview_image', preview), ('ActiveStorage::VariantRecord', 'image', image),
    ]
    assert attachments[0]['record_id'] == case['state']['message']['id']
    assert attachments[1]['record_id'] == source
    assert attachments[2]['record_id'] > 0


def check_dispatch(names, source):
    # A match arm can contain alternatives, e.g. register | unregister.
    branches = set(re.findall(r'"([a-z_]+)"\s*(?:\||=>)', source))
    assert names <= branches, names - branches


def main():
    root = Path(__file__).resolve().parents[2]
    scratch = Path(sys.argv[1]).resolve()
    for kind, filename in [('HTTP', 'agent_http.json'), ('MCP', 'agent_mcp.json'), ('surface', 'agent_surface.json'), ('bot', 'agent_bot_http.json'), ('legacy bot/fanout/replacement', 'agent_legacy_bot_http.json'), ('conversation', 'agent_conversation_http.json'), ('Fizzy reads', 'agent_fizzy_http.json'), ('Fizzy approvals', 'agent_fizzy_action_http.json'), ('readers', 'agent_reads_http.json'), ('pins', 'agent_pins_http.json'), ('polls', 'agent_polls_http.json'), ('polling', 'agent_polling_http.json'), ('reactions', 'agent_reactions_http.json'), ('bot reactions', 'agent_bot_reactions_http.json'), ('work validation', 'agent_work_validation_http.json'), ('work writes', 'agent_work_writes_http.json'), ('attachments', 'agent_attachments_http.json'), ('permissions', 'agent_permissions_http.json'), ('PR192 zones and ID shapes', 'agent_review192_http.json'), ('JPEG commit and processing success', 'agent_review192r2_attachment.json'), ('video commit and processing success', 'agent_review192r3_attachment.json'), ('PR192 handled missing representations', 'agent_review192r5_representations.json'), ('array lookups', 'agent_array_reads_http.json'), ('blob proxy all headers', 'agent_blob_proxy_headers.json')]:
        expected = (root / 'vectors' / filename).read_bytes()
        actual = (scratch / filename).read_bytes()
        compare_vectors(actual, expected, filename)
        if filename == 'agent_review192r2_attachment.json':
            check_attachment_success(json.loads(actual))
        if filename == 'agent_review192r3_attachment.json':
            check_video_attachment_success(json.loads(actual))
        print(f'WS11-api fresh {kind} oracle: {len(json.loads(actual)["cases"])} request/response pairs; byte-identical committed vectors')
    for filename, key in [('agent_id_casting.json', 'cases'), ('agent_budget_notice_reader.json', 'results'), ('agent_array_shapes.json', 'cases'), ('pr214_id_corpus.json', 'cases'), ('next4_numeric_ids.json', 'cases'), ('pr214_event_clock.json', 'rows')]:
        actual = (scratch / filename).read_bytes()
        compare_vectors(actual, (root / 'vectors' / filename).read_bytes(), filename)
        value = json.loads(actual)
        print(f'WS11-api fresh {filename}: {len(value[key])} groups; byte-identical committed vector')
    array_reads = json.loads((scratch / 'agent_array_reads_http.json').read_bytes())
    assert all(case['cache_hits'] == 0 for case in array_reads['cases'])
    print('WS11-api uncached array oracle: 24 cases; zero query-cache hits')
    from proxy_headers import checked
    for filename in ['agent_review192r5_representations.json', 'agent_blob_proxy_headers.json']:
        value=json.loads((scratch / filename).read_bytes())
        for case in value['cases']:
            responses=[case] if 'headers' in case else [v for v in case.values() if isinstance(v,dict) and 'headers' in v]
            for response in responses:
                checked(response['headers'])
                assert response['request_protocol']=='HTTP/1.1'
    print('WS11-api all-header oracle: 25 responses; every header name/value/cardinality; config.ru HTTP/1.1; only 6 named security additions and 3 per-request names approved')
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
