#!/usr/bin/env python3
"""Inject the reviewer's real inline POST, without changing jobs or assertions.

Install in a private clone, compile once, run all three delivery declarations,
then restore the producer byte for byte. Logical URL/Host/path stay configured;
only TCP goes to the per-test recorder started before the producer.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
FILE = ROOT / 'rust/crates/db/src/models/agent_work_events.rs'
ANCHOR = '    Ok(event)\n}\n\npub fn record_owner_change('
REPLACEMENT = '    pr227_inline_post(tx.conn(), &event, thread);\n' + ANCHOR
CASES = ('assignment_webhook', 'assignment_no_read', 'handoff_webhook')
ASSERTION = 'assignment must not block on HTTP before draining jobs'
HOOK = r'''
// Deliberate producer defect installed only in the private control clone.
fn pr227_inline_post(conn: &crate::Connection, event: &AgentEvent, thread: &ChannelThread) {
    let Ok(case) = std::env::var("WS11_PR227_INLINE") else { return; };
    let selected = match case.as_str() {
        "assignment_webhook" => event.event_type == "work_assigned"
            && event.agent_id == 773018776 && thread.name == "Hooked work",
        "assignment_no_read" => event.event_type == "work_assigned"
            && event.agent_id == 773018776 && thread.name == "Unread work",
        "handoff_webhook" => event.event_type == "work_handed_off" && event.agent_id == 1901100002,
        _ => panic!("unknown inline control"),
    };
    if !selected || thread.id != 1900700020 { return; }
    let address: String = conn.query_row("SELECT address FROM ws11_next6_http_observer", [],
        |row| row.get(0)).expect("recorder must exist before the actual producer");
    let address: std::net::SocketAddr = address.parse().unwrap();
    assert_eq!(address.ip(), std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
    let url: String = conn.query_row(
        "SELECT url FROM webhooks WHERE user_id=(SELECT user_id FROM agents WHERE id=?)",
        [event.agent_id], |row| row.get(0)).unwrap();
    assert_eq!(url, "http://93.184.216.34:8080/hook");
    let (authority, path) = url.strip_prefix("http://").unwrap().split_once('/').unwrap();
    let body = json!({"event_id":event.id,"agent_id":event.agent_id,"event_type":event.event_type,
        "thread_id":thread.id,"title":thread.name,"room_id":thread.room_id,
        "assigned_by":event.metadata.get("assigned_by"),"configured_webhook_url":url}).to_string();
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect_timeout(&address, std::time::Duration::from_secs(2)).unwrap();
    stream.set_read_timeout(Some(std::time::Duration::from_secs(2))).unwrap();
    stream.set_write_timeout(Some(std::time::Duration::from_secs(2))).unwrap();
    let head = format!("POST /{path} HTTP/1.1\r\nHost: {authority}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len());
    stream.write_all(head.as_bytes()).unwrap();
    stream.write_all(body.as_bytes()).unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    assert!(response.starts_with(b"HTTP/1.1 200"));
    eprintln!("PR227_INLINE_PRODUCER_HIT {case} event={} thread={} configured_url={url}", event.id, thread.id);
}
'''


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['install', 'run', 'restore'])
    parser.add_argument('--scratch', type=Path, required=True)
    parser.add_argument('--binary', type=Path)
    args = parser.parse_args()
    scratch = args.scratch.resolve()
    if args.action == 'install':
        scratch.mkdir(parents=True, exist_ok=True)
        assert not (scratch / 'original.rs').exists(), 'preserve previous backups'
        original = FILE.read_bytes()
        assert original.decode().count(ANCHOR) == 1 and 'WS11_PR227_INLINE' not in original.decode()
        changed = (original.decode().replace(ANCHOR, REPLACEMENT) + HOOK).encode()
        (scratch / 'original.rs').write_bytes(original)
        (scratch / 'source-hashes.json').write_text(json.dumps(
            {'original': digest(original), 'instrumented': digest(changed)}, indent=2) + '\n')
        FILE.write_bytes(changed)
        print('PR227 inline control: actual assignment/handoff producer instrumented; jobs and facts unchanged')
    elif args.action == 'restore':
        hashes = json.loads((scratch / 'source-hashes.json').read_text())
        assert digest(FILE.read_bytes()) == hashes['instrumented'], 'refuse overwriting intervening edits'
        original = (scratch / 'original.rs').read_bytes()
        assert digest(original) == hashes['original']
        FILE.write_bytes(original)
        print('PR227 inline control: producer restored byte-identical')
    else:
        assert args.binary and args.binary.is_file()
        binary = args.binary.resolve()
        names = subprocess.check_output([binary, '--list'], text=True).splitlines()
        receipts = []
        for case in CASES:
            test, = [s[:-6] for s in names if s.endswith(f'::ws11_next6_{case}: test')]
            for enabled in (False, True):
                env = dict(os.environ, CI='1')
                env.pop('WS12_ASSERTION_MUTATION', None)
                env.pop('WS11_PR227_INLINE', None)
                if enabled:
                    env['WS11_PR227_INLINE'] = case
                command = [str(binary), test, '--exact', '--test-threads=8', '--nocapture']
                result = subprocess.run(command, cwd=ROOT / 'rust/crates/campfire', env=env,
                                        capture_output=True, text=True, timeout=300)
                output = result.stdout + result.stderr
                log = scratch / f'{case}-{"mutant" if enabled else "baseline"}.log'
                log.write_text(output)
                summaries = re.findall(r'^test result:.*$', output, re.M)
                hits = output.count(f'PR227_INLINE_PRODUCER_HIT {case} ')
                assert len(summaries) == 1
                if enabled:
                    assert result.returncode and hits == 1 and ASSERTION in output, output
                    assert '0 passed; 1 failed;' in summaries[0]
                    assert '93.184.216.34:8080' in output and '/hook' in output
                else:
                    assert result.returncode == 0 and hits == 0 and '1 passed; 0 failed;' in summaries[0], output
                receipts.append(dict(case=case, enabled=enabled, command=command, exit=result.returncode,
                    hits=hits, assertion=ASSERTION, summaries=summaries, log=log.name,
                    log_sha256=digest(log.read_bytes())))
                print(f'PR227_INLINE {case} {"mutant" if enabled else "baseline"}: hits={hits}; exit={result.returncode}; {summaries[0]}', flush=True)
        (scratch / 'receipts.json').write_text(json.dumps(receipts, indent=2) + '\n')
        print('PR227 inline controls: 3 baseline passes; 3 real producer POSTs rejected before job draining')


if __name__ == '__main__':
    main()
