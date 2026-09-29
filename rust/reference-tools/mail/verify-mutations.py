#!/usr/bin/env python3
"""Prove security tests discriminate by temporarily breaking one production check at a time."""
from pathlib import Path
import subprocess
import re
ROOT = Path(__file__).resolve().parents[2]
CASES = [
    ('throttle', 'crates/mail/src/inbound.rs', '*count > 30', 'false', 'inbound', 'room_rate_limit_is_thirty_and_resets_each_hour'),
    ('attachment', 'crates/mail/src/parse.rs', 'Verdict::Disallowed', 'Verdict::Ok', 'inbound', 'executable_is_named'),
    ('room-filter', 'crates/mail/src/inbound.rs', "AND deleted_at IS NULL AND type NOT IN ('Rooms::Direct', 'Rooms::Board')", '', 'inbound', 'deleted_room_drops'),
    ('auth-order', 'crates/mail/src/parse.rs', r'headers\s*\.iter\(\)\s*\.find', 'headers.iter().rev().find', 'security', 'topmost_matching_relay_header_wins'),
    ('html-removal', 'crates/mail/src/parse.rs', '"script" | "style" | "head" | "template"', '"ws10-disabled-removal"', 'goldens', 'html_text_matches_nokogiri_corpus'),
    ('atomic-relay', 'crates/mail/src/inbound.rs', 'tx.emit_after_commit(Event::job(&RoutingJob {inbound_email_id: id}));', '', 'http', 'ws10_relay_http_enqueue_failure_rolls_back_acceptance'),
]
logs = ROOT / 'target/ws10-logs'
logs.mkdir(parents=True, exist_ok=True)
for name, file, good, bad, suite, test in CASES:
    path = ROOT / file
    original = path.read_text()
    if (name == 'auth-order' and not re.search(good, original)) or (name != 'auth-order' and good not in original):
        raise SystemExit(f'{name}: mutation target missing')
    try:
        path.write_text(re.sub(good, bad, original, count=1) if name == "auth-order" else original.replace(good, bad, 1))
        command = ['mise', 'exec', 'rust@1.98.1', '--', 'cargo', 'test', '-p', 'campfire' if suite == 'http' else 'campfire_mail']
        if suite != 'http':
            command += ['--test', suite]
        command += ['-j', '4', test]
        result = subprocess.run(command, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (logs / f'mutation-{name}.log').write_text(result.stdout)
        summaries = [line for line in result.stdout.splitlines() if line.startswith('test result:')]
        if result.returncode == 0 or not summaries or 'FAILED' not in summaries[-1]:
            raise SystemExit(f'{name}: mutation did not fail its test: {result.stdout}')
        print(f'{name}: {summaries[-1]}', flush=True)
    finally:
        path.write_text(original)
