#!/usr/bin/env python3
"""Reproduce the flagged, unresolved Rails after-commit JPEG difference; require failure."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
ROOT = Path(__file__).resolve().parents[3]
clone = Path(tempfile.mkdtemp(prefix='ws8bm-image-gap-', dir=ROOT / '.scratch'))
subprocess.run(['git', 'clone', '--quiet', '--shared', str(ROOT), str(clone)], check=True)
revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=clone, text=True).strip()
shutil.copyfile(ROOT / 'rust/vectors/messaging/thread-upload-coverage.json', clone / 'rust/vectors/messaging/thread-upload-coverage.json')
module = clone / 'rust/crates/campfire/src/controllers/messages/review_tests.rs'
with module.open('a') as source:
    source.write('''
#[tokio::test]
async fn initial_jpeg_after_commit_gap() {
    let app = app().await;
    let oracle: Value = serde_json::from_str(include_str!("../../../../../vectors/messaging/thread-upload-coverage.json")).unwrap();
    let row = oracle["rows"].as_array().unwrap().iter().find(|row| row["name"] == "top_image").unwrap();
    let input = &row["responses"][0];
    let before = app.db().read(|conn| Ok((conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?, conn.query_row("SELECT COUNT(*) FROM channel_threads", [], |r| r.get::<_, i64>(0))?))).await.unwrap();
    let response = app.david().write(Req::new(Method::POST, input["path"].as_str().unwrap()).header("content-type", "application/json").body(input["input"].to_string())).await;
    let after = app.db().read(|conn| Ok((conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?, conn.query_row("SELECT COUNT(*) FROM channel_threads", [], |r| r.get::<_, i64>(0))?))).await.unwrap();
    println!("WS8bm initial JPEG gap: Rust status {}; new messages {}; new threads {}; Rails status {} / messages {} / threads {}", response.status, after.0-before.0, after.1-before.1, input["status"], row["clients"].as_array().unwrap().len(), row["thread_count"]);
    assert_eq!(response.status.as_u16(), input["status"].as_u64().unwrap() as u16, "Flagged Rails variant after_commit closed-stream failure; no parity acceptance");
}
''')
(clone / '.scratch').mkdir()
env = dict(os.environ, CI='1', TMPDIR=str(clone / '.scratch'), CARGO_TARGET_DIR=str(ROOT / 'rust/target'), CAMPFIRE_REFERENCE=str(clone),
           CABLE_TEST_PORT_RANGE='52000-52049', MAIL_TEST_PORT_RANGE='52000-52049', PARITY_IMAGE='triage-reference-d7c7de92', PARITY_NAMESPACE='ws8bm-image-gap', PARITY_OWNER='ws8bm', PARITY_CPUS='2')
env.pop('RUST_TEST_THREADS', None)
print(f'WS8bm image-gap checkout: {revision}; {clone}', flush=True)
for name, cmd in [('seeds', ['bash', 'rust/parity/bin/seed', 'build', 'default', 'first_run']), ('test', ['mise', 'exec', 'rust@1.98.1', '--', 'cargo', 'test', '--locked', '-j4', '--manifest-path', 'rust/Cargo.toml', '-p', 'campfire', '--bin', 'campfire', 'initial_jpeg_after_commit_gap', '--', '--nocapture'])]:
    with (clone / '.scratch' / f'{name}.log').open('w') as log:
        result = subprocess.run(cmd, cwd=clone, env=env, stdout=log, stderr=subprocess.STDOUT)
    output = (clone / '.scratch' / f'{name}.log').read_text()
    for line in output.splitlines():
        if line.startswith(('seed:', '    Finished', 'WS8bm', 'test result:')): print(line, flush=True)
    if name == 'seeds': assert result.returncode == 0, output[-4000:]
    else: assert result.returncode == 101 and 'test result: FAILED. 0 passed; 1 failed;' in output, output[-4000:]
print('WS8bm initial JPEG: unresolved mismatch reproduced; explicit deferred case, not an application-suite ignore or parity mask', flush=True)
