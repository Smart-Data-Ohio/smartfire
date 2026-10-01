#!/usr/bin/env python3
"""Actual compiled regressions for reply parenting, guard, winner adoption and refs."""
from pathlib import Path
import os,subprocess
root=Path(__file__).resolve().parents[3]
env=dict(os.environ,CARGO_BUILD_JOBS='2',CI='1',TMPDIR=str(root/'.scratch'),CABLE_TEST_PORT_RANGE='52200-52249',MAIL_TEST_PORT_RANGE='52200-52249',INTEGRATION_TEST_PORT_RANGE='52250-52299')
cases=[
 ('reply-parent','rust/crates/campfire/src/integrations/jobs.rs','attributes.reply_to_message_id = trigger.map(|message| message.id);','attributes.reply_to_message_id = None;','campfire','ws11_webhook_case_root_references_trigger'),
 ('private-guard','rust/crates/campfire/src/integrations/webhook.rs','crate::integrations::net::guard::resolve_webhook(&*net.resolver, &host).await?','"93.184.216.34".parse().unwrap()','campfire','ws11_webhook_case_loopback_refused'),
 ('winner-secret','rust/crates/db/src/models/webhook.rs','*self = query_one(tx.conn(), "SELECT webhooks.* FROM webhooks WHERE id = ? LIMIT 1", [self.id], Self::from_row)?.ok_or(Error::RecordNotFound("Webhook"))?;','// deliberately retain the stale nil secret','campfire_db','ws11_webhook_case_signing_secret_generation_adopts_winner'),
 ('peer-refs','rust/crates/db/src/models/message.rs','tx.model_callback(Phase::MessageLinkReferences, self.id)?;\n        self.sync_external_references(tx, true)','tx.model_callback(Phase::MessageLinkReferences, self.id)?;\n        Ok(())','campfire','ws11_stream_case_link_references_sync_only_at_finalize'),
]
for name,file,before,after,package,test in cases:
 p=root/file;original=p.read_text();assert original.count(before)==1,(name,original.count(before))
 log=root/'.scratch'/f'installed-webhook-mutation-{name}.log'
 try:
  p.write_text(original.replace(before,after))
  with log.open('w') as out:
   run=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j2','--manifest-path','rust/Cargo.toml','-p',package,test,'--','--test-threads=4'],cwd=root,env=env,stdout=out,stderr=subprocess.STDOUT)
  summaries=[s for s in log.read_text().splitlines() if s.startswith('test result:')]
  assert run.returncode==101 and any('FAILED. 0 passed; 1 failed;' in s for s in summaries),(name,log)
  print(f'WS11 installed webhook mutation: {name}; exit={run.returncode}; '+summaries[-1],flush=True)
 finally:p.write_text(original)
print('WS11 installed webhook mutations: 4 compiled mutations caught; source restored')
