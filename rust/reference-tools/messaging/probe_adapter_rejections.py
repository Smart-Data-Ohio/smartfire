#!/usr/bin/env python3
"""Record the owner-blocked adapter contract beside its real atomic Rust output.

This diagnostic is not a passing parity gate. The Rails adapter has no corresponding
soft-refusal API on Rust's durable queue. --control suppresses the actual durable
enqueue error and must fail at the refusal assertion. No workaround is installed.
Temporary module/source bytes are restored; complete actual rows are recorded.
"""
from pathlib import Path
import argparse
import json
import subprocess
ROOT = Path(__file__).resolve().parents[3]
p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--control', action='store_true')
p.add_argument('--output', type=Path, required=True)
p.add_argument('runner', nargs=argparse.REMAINDER)
a = p.parse_args()
runner = a.runner[1:] if a.runner[:1] == ['--'] else a.runner
assert runner
module = ROOT / 'rust/crates/campfire/src/controllers/message_features/adapter_rejection_probe.rs'
assert not module.exists()
parent = ROOT / 'rust/crates/campfire/src/controllers/message_features.rs'
jobs = ROOT / 'rust/crates/campfire/src/jobs.rs'
original = {parent: parent.read_bytes(), jobs: jobs.read_bytes()}
try:
    module.write_text(r'''
use super::{comparison_support, quote_integration_tests::app_rows};
use crate::integrations::link_embed::{Embed,metadata_parser::Metadata};
use serde_json::{Value,json};
#[tokio::test]
async fn adapter_owner_contract_probe() {
 let oracle:Value=serde_json::from_str(include_str!("../../../../../vectors/messaging/adapter_rejections.json")).unwrap();
 let mut counts=std::collections::HashMap::new();let mut records=vec![];
 for group in oracle["groups"].as_array().unwrap() {
  for fail_at in [1,2] {
   let app=app_rows(group["rows"].clone()).await.without_job_runner().await;
   let (mut client,server)=super::comparison_support::embed_streams(&app,group["thread_id"].as_i64().unwrap()).await;
   app.db().write(move|tx|{tx.conn().execute_batch(&format!("DELETE FROM background_jobs; CREATE TRIGGER ws8_adapter_failure BEFORE INSERT ON background_jobs WHEN NEW.job_class='LinkEmbed::FetchJob' AND (SELECT COUNT(*) FROM background_jobs WHERE job_class='LinkEmbed::FetchJob')>={} BEGIN SELECT RAISE(ABORT,'fixture adapter write failure'); END",fail_at-1))?;Ok(())}).await.unwrap();
   let id=group["embed_id"].as_i64().unwrap();let log=app.db().capture_queries();
   let result=app.db().write(move|tx|{let embed=Embed::find(tx.conn(),id)?;embed.save_metadata(tx,&Metadata{title:Some("After adapter refusal".into()),description:embed.description.clone(),site_name:embed.site_name.clone(),image_url:embed.image_url.clone()})}).await;
   app.db().stop_capturing_queries();let reads=log.lock().unwrap().len();
   assert!(result.is_err_and(|e|e.to_string().contains("fixture adapter write failure")),"adapter actual durable refusal");
   let published=app.publications().take();assert!(published.is_empty(),"adapter actual rollback publications");client.assert_silent().await;
   let before=group["rows"].clone();
   let actual=app.db().read(move|conn|{
    let mut state=serde_json::Map::new();
    for (table,expected) in before.as_object().unwrap() {
     let ids=expected.as_array().unwrap().iter().map(|r|r["id"].clone()).collect::<Vec<_>>();
     let ids=conn.prepare(&format!("SELECT id FROM {table} WHERE id IN (SELECT value FROM json_each(?)) ORDER BY id"))?.query_map([json!(ids).to_string()],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
     let rows=ids.into_iter().map(|id|comparison_support::row(conn,table,id)).collect::<campfire_db::Result<Vec<_>>>()?;
     assert_eq!(rows.len(),expected.as_array().unwrap().len(),"adapter actual rollback row count");
     for (actual,expected) in rows.iter().zip(expected.as_array().unwrap()){comparison_support::same_row(actual,expected,"adapter actual rollback rows");}
     state.insert(table.clone(),json!(rows));
    }
    let pending=conn.query_row("SELECT COUNT(*) FROM background_jobs",[],|r|r.get::<_,i64>(0))?;assert_eq!(pending,0,"adapter actual partial durable queue rollback");
    Ok(json!({"state":state,"pending":pending,"publications":published}))
   }).await.unwrap();
   let rails=group["cases"].as_array().unwrap().iter().find(|c|c["mode"]=="runtime_error"&&c["fail_at"]==fail_at).unwrap();
   assert_ne!(actual["state"],rails["state"],"do not claim adapter parity: Rails committed metadata/claims, Rust rolled back");
   let key=format!("{} refusal {fail_at}",group["kind"].as_str().unwrap());
   println!("WS8bm2 adapter owner probe {key} size={}: Rust {reads}; Rails {}; actual Rust atomic rollback differs from Rails after-commit refusal",group["size"],rails["reads"]);
   if let Some(previous)=counts.insert(key,reads){assert_eq!(previous,reads,"adapter physical read growth");}
   records.push(json!({"kind":group["kind"],"size":group["size"],"fail_at":fail_at,"reads":reads,"actual":actual}));server.abort();
  }
 }
 println!("WS8bm2 adapter actual JSON {}",json!(records));
 println!("WS8bm2 adapter owner probe: 8/8 atomic refusal executions; complete actual rows and empty publications; flat reads; Rails adapter parity remains owner-blocked");
}
''')
    parent.write_bytes(original[parent] + b'\n#[cfg(test)]\nmod adapter_rejection_probe;\n')
    if a.control:
        marker = 'let id = self.queue.enqueue(tx, &request)?;'
        text = jobs.read_text()
        assert text.count(marker) == 1
        jobs.write_text(text.replace(marker, 'let id = self.queue.enqueue(tx, &request).unwrap_or(0);'))
    r = subprocess.run(runner + ['test', '--locked', '-p', 'campfire', '--bin', 'campfire',
                                 'adapter_owner_contract_probe', '-j2', '--', '--test-threads=4', '--nocapture'],
                       cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    a.output.parent.mkdir(parents=True, exist_ok=True)
    a.output.with_suffix('.log').write_text(r.stdout)
    for line in r.stdout.splitlines():
        if line.startswith(('WS8bm2 adapter owner', 'test result:')):
            print(line, flush=True)
    if a.control:
        assert r.returncode != 0 and '\nadapter actual durable refusal\n' in r.stdout, r.stdout[-2000:]
        print('WS8bm2 adapter enqueue-error producer mutant: rejected at adapter actual durable refusal', flush=True)
    else:
        assert r.returncode == 0, r.stdout[-2000:]
        line = next(l for l in r.stdout.splitlines() if l.startswith('WS8bm2 adapter actual JSON '))
        a.output.write_text(json.dumps(json.loads(line.removeprefix('WS8bm2 adapter actual JSON ')), indent=2) + '\n')
finally:
    for path, raw in original.items():
        path.write_bytes(raw)
    module.unlink(missing_ok=True)
