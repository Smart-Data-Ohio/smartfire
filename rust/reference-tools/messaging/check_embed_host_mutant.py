#!/usr/bin/env python3
"""Inject the review's real outgoing Host defect only in http_error jobs.

Run in this worker's idle checkout, passing its configured Cargo runner after --.
--expect legacy records the old comparator's escape; --expect reject proves the
received-header comparator fails. All actual jobs/outcomes still execute. Source
is restored byte for byte; no mutation selector is kept in production code.
"""
import argparse
from pathlib import Path
import subprocess

ROOT=Path(__file__).resolve().parents[3]
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--expect',choices=['legacy','reject'],required=True)
p.add_argument('runner',nargs=argparse.REMAINDER)
a=p.parse_args();runner=a.runner[1:] if a.runner[:1]==['--'] else a.runner
assert runner
originals={}
def replace(relative,old,new):
    path=ROOT/relative;raw=path.read_bytes();text=raw.decode();assert text.count(old)==1,(relative,old)
    originals[path]=raw;path.write_text(text.replace(old,new))
try:
    replace('rust/crates/campfire/src/net/http.rs', 'pub const NET_HTTP_DEFAULT_TIMEOUT:',
            'pub(crate) static EMBED_HOST_FAULT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);\npub const NET_HTTP_DEFAULT_TIMEOUT:')
    path=ROOT/'rust/crates/campfire/src/net/http.rs';text=path.read_text()
    marker='self.default("Host", &endpoint.host_header());';assert text.count(marker)==1
    path.write_text(text.replace(marker,marker+'''\n        if EMBED_HOST_FAULT.load(std::sync::atomic::Ordering::SeqCst) {
            for (name, value) in &mut self.headers {
                if name.eq_ignore_ascii_case("host") { *value = "wrong-host.example.test".into(); }
            }
        }'''))
    for name in ['older_embed_job_tests','older_embed_children_tests']:
        replace(f'rust/crates/campfire/src/controllers/message_features/{name}.rs',
            'for job in group["jobs"].as_array().unwrap() {',
            '''for job in group["jobs"].as_array().unwrap() {
            crate::net::http::EMBED_HOST_FAULT.store(job["name"]=="http_error",std::sync::atomic::Ordering::SeqCst);''')
        path=ROOT/f'rust/crates/campfire/src/controllers/message_features/{name}.rs';text=path.read_text()
        marker='assert!(r.header("Authorization").is_none());';assert text.count(marker)==1
        path.write_text(text.replace(marker,marker+'''\n                    println!("WS8bm2 Host control case={} observed={:?}",job["name"],r.header("Host"));'''))
    for name,wrong_count in [('older_embed_job_tests',4),('older_embed_children_tests',8)]:
        result=subprocess.run(runner+['test','--locked','-p','campfire','--bin','campfire',name,'-j2','--','--test-threads=4','--nocapture'],cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
        print(result.stdout,flush=True)
        if a.expect=='legacy':
            assert result.returncode==0 and result.stdout.count('observed=Some("wrong-host.example.test")')==wrong_count,f'{name}: did not complete the undetected fault'
        else:
            assert result.returncode!=0 and 'wrong-host.example.test' in result.stdout and 'assertion `left == right` failed: actual wire HTTP calls' in result.stdout,f'{name}: Host fault escaped or failed before comparison'
    print(f'WS8bm2 HTTP Host mutant: both full matrices '+('escaped the legacy comparison' if a.expect=='legacy' else 'rejected by received-header comparisons'),flush=True)
finally:
    for path,raw in originals.items():path.write_bytes(raw)
