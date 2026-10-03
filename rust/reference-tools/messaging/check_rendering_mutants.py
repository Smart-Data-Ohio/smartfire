#!/usr/bin/env python3
"""Corrupt real producers, preserving every input, fixture, receiver and deadline.

One temporary selector allows one compilation for the controls. Each selected
producer has an intended independently captured output witness. Restores bytes.
"""
from pathlib import Path
import re,subprocess,sys
ROOT=Path(__file__).resolve().parents[3]
runner=sys.argv[1:]
if runner[:1]==['--']:runner=runner[1:]
assert runner
originals={}
def replace(relative,old,new):
 p=ROOT/relative;raw=p.read_bytes();s=raw.decode();assert s.count(old)==1,(relative,old,s.count(old))
 originals.setdefault(p,raw);p.write_text(s.replace(old,new))
selector='rails_compat::datetime::WS8_RENDER_MUTANT.load(std::sync::atomic::Ordering::SeqCst)'
try:
 replace('rust/crates/rails_compat/src/datetime.rs','pub trait TimeValue:', 'pub static WS8_RENDER_MUTANT: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);\npub trait TimeValue:')
 replace('rust/crates/views/src/messages.rs','            &ctx.time_zone,\n            self.created_at,',f'            &if {selector}==1 {{crate::time::Zone::utc()}} else {{ctx.time_zone.clone()}},\n            self.created_at,')
 replace('rust/crates/campfire/src/controllers/presenters.rs','        view.avatar_url = avatar_path_in_zone(self.secrets, &user, &self.render_zone);',f'        view.avatar_url = if {selector}==2 {{avatar_path(self.secrets, &user)}} else {{avatar_path_in_zone(self.secrets, &user, &self.render_zone)}};')
 replace('rust/crates/campfire/src/controllers/message_features.rs','    rails_compat::datetime::render(at, zone.tz(), true)',f'    if {selector}==3 {{return "mutated actual HTML timestamp".into();}}\n    rails_compat::datetime::render(at, zone.tz(), true)')
 replace('rust/crates/views/src/scheduled_messages.rs','.value(&self.send_value)',f'.value(if {selector}==4 {{"mutated actual form date"}} else {{&self.send_value}})')
 replace('rust/crates/campfire/src/controllers/message_features.rs','    rails_compat::datetime::format(at, zone.tz(), "%B %d, %Y %H:%M")',f'    if {selector}==5 {{return "mutated actual scheduling notice".into();}}\n    rails_compat::datetime::format(at, zone.tz(), "%B %d, %Y %H:%M")')
 replace('rust/crates/db/src/models/saved_item.rs','params![now, tx.now(), item.id]',f'params![if {selector}==6 {{None}} else {{Some(now)}}, tx.now(), item.id]')
 replace('rust/crates/db/src/models/saved_item.rs','ReminderPushJob { saved_item_id: item.id }',f'ReminderPushJob {{ saved_item_id: if {selector}==7 {{item.id-1}} else {{item.id}} }}')
 replace('rust/crates/db/src/models/scheduled_message.rs','if !immediate && scheduled.send_at > now {',f'if !immediate && {selector}!=8 && scheduled.send_at > now {{')
 replace('rust/crates/campfire/src/jobs/periodic.rs','|app: App| async move { scheduled_messages(&app.db).await }',f'|app: App| async move {{ if {selector}==9 {{Ok(())}} else {{scheduled_messages(&app.db).await}} }}')
 replace('rust/crates/campfire/src/channels/message_features.rs','    cable.broadcast_stream_to(&[&broadcast.stream_name()], &html);',f'    let html = if {selector}==10 {{html.replace(r#"action="append""#,r#"action="replace""#)}} else {{html}};\n    cable.broadcast_stream_to(&[&broadcast.stream_name()], &html);')
 replace('rust/crates/campfire/src/jobs/periodic.rs','for task in message_delivery_tasks(intervals.reminders) { periodic.task(task); }',f'for task in message_delivery_tasks(intervals.reminders) {{ if {selector}!=11 || task.name()!="scheduled messages" {{periodic.task(task);}} }}')
 replace('rust/crates/views/src/messages.rs','format!("{}/zone/{}", message_fragment_key(id, updated_at, base_url, stamp), zone.name())',f'format!("{{}}/zone/{{}}", message_fragment_key(id, updated_at, base_url, stamp), if {selector}==12 {{"UTC"}} else {{zone.name()}})')
 replace('rust/crates/views/src/messages.rs','self.details.edited_at.map(|at| ctx.time_zone.iso8601(at)).unwrap_or_default()',f'self.details.edited_at.map(|at| if {selector}==13 {{iso8601(at)}} else {{ctx.time_zone.iso8601(at)}}).unwrap_or_default()')
 modules={
  'container_input_tests.rs':'exceptional_relative_consumers_match_rails_complete_state_with_flat_reads',
  'wide_html_tests.rs':'wide_saved_and_scheduled_html_match_rails_with_flat_reads',
  'periodic_delivery_tests.rs':'periodic_wide_and_due_delivery_match_rails_full_rows_jobs_and_frames',
  '../messages/declaration_tests.rs':'root_edit_markers_match_rails_for_noops_attachments_formatting_reactions_fetches_tombstones_and_zones',
 }
 for module,test in modules.items():
  replace('rust/crates/campfire/src/controllers/message_features/'+module,'#[tokio::test]\nasync fn '+test+'()', 'async fn '+test+'()')
 replace('rust/crates/campfire/src/controllers/message_features/wide_html_tests.rs','#[tokio::test]\nasync fn wide_saved_and_scheduled_http_dates_and_notices_match_rails()', 'async fn wide_saved_and_scheduled_http_dates_and_notices_match_rails()')
 controls=[
  (1,'container_input_tests.rs',modules['container_input_tests.rs'],'warm zone actual legacy fragment differs from Rails: America/New_York'),
  (2,'container_input_tests.rs',modules['container_input_tests.rs'],'warm zone actual legacy fragment differs from Rails: America/New_York'),
  (3,'wide_html_tests.rs',modules['wide_html_tests.rs'],'wide HTML actual saved partial differs from Rails'),
  (4,'wide_html_tests.rs',modules['wide_html_tests.rs'],'wide HTML actual scheduled partial differs from Rails'),
  (5,'wide_html_tests.rs','wide_saved_and_scheduled_http_dates_and_notices_match_rails','wide HTML actual HTTP notice differs from Rails'),
  (6,'periodic_delivery_tests.rs',modules['periodic_delivery_tests.rs'],'periodic actual persisted rows.reminded_at'),
  (7,'periodic_delivery_tests.rs',modules['periodic_delivery_tests.rs'],'periodic actual durable jobs differ from Rails'),
  (8,'periodic_delivery_tests.rs',modules['periodic_delivery_tests.rs'],'periodic actual persisted row count: messages'),
  (9,'periodic_delivery_tests.rs',modules['periodic_delivery_tests.rs'],'periodic actual persisted row count:'),
  (10,'periodic_delivery_tests.rs',modules['periodic_delivery_tests.rs'],'periodic actual publications'),
  (11,'periodic_delivery_tests.rs',modules['periodic_delivery_tests.rs'],'periodic actual registered ticks differ from Rails'),
  (12,'container_input_tests.rs',modules['container_input_tests.rs'],'warm zone actual legacy fragment differs from Rails: America/New_York'),
  (13,'../messages/declaration_tests.rs',modules['../messages/declaration_tests.rs'],'actual viewer-zone meta differs from Rails: Pacific Time (US & Canada)'),
 ]
 for n,module,test,witness in controls:
  with (ROOT/'rust/crates/campfire/src/controllers/message_features'/module).open('a') as f:
   f.write(f'\n#[tokio::test] async fn ws8_render_producer_mutant_{n}() {{ rails_compat::datetime::WS8_RENDER_MUTANT.store({n}, std::sync::atomic::Ordering::SeqCst); {test}().await; }}\n')
 for n,_,_,witness in controls:
  r=subprocess.run(runner+['test','--locked','-p','campfire','--bin','campfire',f'ws8_render_producer_mutant_{n}','-j2','--','--test-threads=4','--nocapture'],cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  print(r.stdout,flush=True)
  assert r.returncode!=0 and re.search(r"panicked at [^\n]+\n(?:assertion `left == right` failed: )?"+re.escape(witness),r.stdout),(n,witness)
  print(f'WS8bm2 rendering producer mutant {n}: rejected at {witness}',flush=True)
finally:
 for p,raw in originals.items():p.write_bytes(raw)
