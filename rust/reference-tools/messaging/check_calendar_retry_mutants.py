#!/usr/bin/env python3
"""Reject actual Calendar producer faults at their respective output assertions.

Sources are restored byte-for-byte. Only one mutation test runs per process; the
temporary producer selector never ships. Fixture, receiver, clock, query counts
and deadlines are untouched. Run with a configured Cargo runner after --.
"""
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
runner = sys.argv[1:]
if runner[:1] == ['--']:
    runner = runner[1:]
assert runner
originals = {}

def replace(relative, old, new):
    path = ROOT / relative
    raw = path.read_bytes()
    text = raw.decode()
    assert text.count(old) == 1, (relative, old)
    originals.setdefault(path, raw)
    path.write_text(text.replace(old, new))

selector = 'campfire_db::models::google_entry::WS8_RETRY_MUTANT.load(std::sync::atomic::Ordering::SeqCst)'
try:
    replace('rust/crates/db/src/models/google_entry.rs',
            'pub fn failure(tx: &mut Tx',
            'pub static WS8_RETRY_MUTANT: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);\npub fn failure(tx: &mut Tx')
    replace('rust/crates/db/src/models/google_entry.rs',
            "pub fn failure(tx: &mut Tx<'_>, entry: &Entry, error: &str) -> Result<()> {",
            "pub fn failure(tx: &mut Tx<'_>, entry: &Entry, error: &str) -> Result<()> {\nif WS8_RETRY_MUTANT.load(std::sync::atomic::Ordering::SeqCst)==1 {return Ok(());}")
    replace('rust/crates/campfire/src/integrations/google/calendar.rs',
            'JobError::retry_group(error, GOOGLE_RETRY, 8)',
            f'JobError::retry_group(error, GOOGLE_RETRY, if {selector}==2 {{7}} else {{8}})')
    replace('rust/crates/campfire/src/integrations/google/entry_sync.rs',
            'let payload = entries::payload(&event, app.config.mail.app_url.as_deref());',
            f'let mut payload = entries::payload(&event, app.config.mail.app_url.as_deref());\nif {selector}==3 {{payload["summary"]=json!("mutated actual owner payload");}}')
    replace('rust/crates/campfire/src/integrations/google/calendar_sync.rs',
            'pub async fn inbound(app: &App, user_id: i64) -> api::Result<()> {',
            f'''pub async fn inbound(app: &App, user_id: i64) -> api::Result<()> {{
if {selector}==4 {{app.cable.broadcast(&format!("{{}}:messages",campfire_views::helpers::gid_param("Rooms::Closed",699448326)),"mutated actual broadcast");}}''')
    path = 'rust/crates/campfire/src/controllers/message_features/calendar_retry_consumer_tests.rs'
    replace(path, '#[tokio::test]\nasync fn calendar_retry_recovery_and_exhaustion_match_real_rails_jobs_with_flat_reads()',
            'async fn calendar_retry_recovery_and_exhaustion_match_real_rails_jobs_with_flat_reads()')
    p = ROOT / path
    witnesses = [(1, 'Calendar persisted entry.last_error'), (2, 'Calendar queue state'),
                 (3, 'Calendar owner exchanges'), (4, 'ordered publication differs from Rails: Calendar retry publications')]
    with p.open('a') as out:
        for number, _ in witnesses:
            out.write(f'''\n#[tokio::test] async fn ws8_calendar_producer_mutant_{number}() {{
campfire_db::models::google_entry::WS8_RETRY_MUTANT.store({number},std::sync::atomic::Ordering::SeqCst);
calendar_retry_recovery_and_exhaustion_match_real_rails_jobs_with_flat_reads().await;
}}\n''')
    for number, witness in witnesses:
        r = subprocess.run(runner + ['test', '--locked', '-p', 'campfire', '--bin', 'campfire',
                                     f'ws8_calendar_producer_mutant_{number}', '-j2', '--', '--test-threads=4', '--nocapture'],
                           cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        print(r.stdout, flush=True)
        assert r.returncode != 0 and f'assertion `left == right` failed: {witness}' in r.stdout, witness
        print(f'WS8bm2 Calendar producer mutant {number}: rejected at {witness}', flush=True)
finally:
    for path, raw in originals.items():
        path.write_bytes(raw)
