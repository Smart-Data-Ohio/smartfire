#!/usr/bin/env python3
"""Reject actual lifecycle defects in the six strengthened original native tests."""
import os,re,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[1];out=root/'target/controller-c-lifecycle-controls';out.mkdir(parents=True,exist_ok=True)
icons='controllers::accounts::icons::tests::';bans='controllers::users::ban_lifecycle_tests::'
controls=[
 ('icon deletion','crates/campfire/src/controllers/accounts/icons.rs','icon.destroy(tx)?;','let _ = &icon;',icons+'destroy_removes_attachment_and_blob_and_records_one_snapshot_audit','original icon destroy count delta'),
 ('invalid name error','crates/db/src/models/workspace_icon.rs','errors.add("name", "is already taken by a built-in icon");','errors.add("name", "mutant");',icons+'invalid_and_duplicate_uploads_return_inline_errors_without_rows_or_audits','already taken by a built-in icon'),
 ('race error','crates/db/src/models/workspace_icon.rs','errors.add("name", "has already been taken");','errors.add("name", "mutant");',icons+'uniqueness_index_races_render_taken_and_roll_back_upload_and_audit','has already been taken'),
 ('session destruction','crates/db/src/models/user.rs','// apply_ban\n        self.close_remote_connections(tx, false);\n        tx.conn().execute_cached(\n            r#"DELETE FROM "sessions" WHERE "sessions"."user_id" = ?"#,','// apply_ban\n        self.close_remote_connections(tx, false);\n        tx.conn().execute_cached(\n            r#"DELETE FROM "sessions" WHERE "sessions"."user_id" = ? AND 0"#,',bans+'banning_a_user_removes_pending_two_factor_setup_and_sessions','left: (1,'),
 ('durable payload','crates/campfire/src/jobs.rs','Event::RemoveBannedContent { user_id } => JobRequest::new(&RemoveBannedContentJob { user_id: *user_id }),','Event::RemoveBannedContent { user_id: _ } => JobRequest::new(&RemoveBannedContentJob { user_id: 0 }),',bans+'ban_http_enqueue_is_atomic_and_writes_one_durable_remove_job','Number(0)'),
 ('message destruction','crates/campfire/src/jobs.rs','for message in messages {','for message in messages.into_iter().take(0) {',bans+'ban_http_removes_the_users_messages_through_the_real_runner',"RemoveBannedContentJob did not delete the user's messages"),
]
originals={}
try:
 for name,rel,before,after,test,marker in controls:
  p=root/rel;s=p.read_text();assert s.count(before)==(2 if name=='race error' else 1),(name,'source guard');originals.setdefault(p,p.read_bytes());p.write_text(s.replace(before,after))
 env=os.environ.copy();env.update(CI='1',RUST_TEST_THREADS='4',CAMPFIRE_REFERENCE=str(root.parent))
 expr=' or '.join('test(='+t+')' for _,_,_,_,t,_ in controls)
 r=subprocess.run(['cargo','nextest','run','--locked','-p','campfire','-j','4','--no-fail-fast','-E',expr],cwd=root,env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
 (out/'mutants.log').write_text(r.stdout)
 assert r.returncode==100 and '6 tests run: 0 passed, 6 failed' in r.stdout,r.stdout[-6000:]
 for name,_,_,_,test,marker in controls:
  assert re.search(r'^\s*FAIL\s+\[[^\]]+\].*'+re.escape(test)+r'$',r.stdout,re.M),(name,'producer survived')
  block=r.stdout.split('test '+test+' ... FAILED',1)[1].split('\n        FAIL',1)[0]
  assert marker in block and ('assertion' in block or name=='message destruction'),(name,'invalid rejection',block[-2000:])
  print(f'C lifecycle control: {name}: intended test rejected',flush=True)
 print('C lifecycle discrimination: 6 producer mutations; 6 tests rejected; 0 invalid controls',flush=True)
finally:
 for p,source in originals.items():p.write_bytes(source)
