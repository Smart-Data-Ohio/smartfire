#!/usr/bin/env python3
"""Mutate actual time, split, HTTP and persisted outputs; require their witnesses.

A temporary selector permits one producer per process and one compile. All source
bytes are restored in finally. No fixture, input, receiver or deadline is changed.
"""
from pathlib import Path
import subprocess
import re
import sys
ROOT=Path(__file__).resolve().parents[3]
runner=sys.argv[1:];runner=runner[1:] if runner[:1]==['--'] else runner
assert runner
originals={}
def replace(relative,old,new):
 p=ROOT/relative;raw=p.read_bytes();s=raw.decode();assert s.count(old)==1,(relative,old)
 originals.setdefault(p,raw);p.write_text(s.replace(old,new))
selector='crate::slash_commands::time_parser::WS8_INPUT_MUTANT.load(std::sync::atomic::Ordering::SeqCst)'
try:
 replace('rust/crates/db/src/slash_commands/time_parser.rs', 'pub const WEEKDAYS:', 'pub static WS8_INPUT_MUTANT: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);\npub const WEEKDAYS:')
 replace('rust/crates/db/src/slash_commands/time_parser.rs','let multiplier = if unit.starts_with("min") { 60 } else { 3600 };',f'let multiplier = if {selector}==1 {{61}} else if unit.starts_with("min") {{60}} else {{3600}};')
 replace('rust/crates/db/src/slash_commands/time_parser.rs','(Some(title.to_owned()), from_match(&c, &zone, now))',f'(Some(if {selector}==2 {{format!("{{title}} mutated")}} else {{title.to_owned()}}), from_match(&c, &zone, now))')
 replace('rust/crates/db/src/slash_commands.rs','CommandResult::error("Type / to see available commands.")',f'CommandResult::error(if {selector}==3 {{"mutated actual slash response"}} else {{"Type / to see available commands."}})')
 replace('rust/crates/db/src/models/saved_item.rs','let status = attributes.status.unwrap_or_else(|| "in_progress".into());',f'let status = attributes.status.unwrap_or_else(|| if {selector}==4 {{"done".into()}} else {{"in_progress".into()}});')

 replace('rust/crates/campfire/src/controllers/message_features.rs',
         '    let mut encoded = rails_compat::datetime::render(time, zone, true);',
         '    if campfire_db::slash_commands::time_parser::WS8_INPUT_MUTANT.load(std::sync::atomic::Ordering::SeqCst)==5 {return "mutated actual timestamp".into();}\n    let mut encoded = rails_compat::datetime::render(time, zone, true);')
 replace('rust/crates/campfire/src/channels/message_features.rs',
         '    cable.broadcast_stream_to(&[&broadcast.stream_name()], &html);',
         '    let html=if campfire_db::slash_commands::time_parser::WS8_INPUT_MUTANT.load(std::sync::atomic::Ordering::SeqCst)==6 {html.replace(r#"action="append""#,r#"action="replace""#)} else {html};\n    cable.broadcast_stream_to(&[&broadcast.stream_name()], &html);')

 replace('rust/crates/db/src/slash_commands.rs',
         'fn wide_long(time: Timestamp, zone_name: &str, clock: bool) -> String {',
         'fn wide_long(time: Timestamp, zone_name: &str, clock: bool) -> String {\nif crate::slash_commands::time_parser::WS8_INPUT_MUTANT.load(std::sync::atomic::Ordering::SeqCst)==8 {return "mutated real notice".into();}')
 relative='rust/crates/campfire/src/controllers/message_features/relative_split_input_tests.rs'
 replace(relative,'#[test]\nfn exceptional_relative_and_split_inputs_match_rails()', 'fn exceptional_relative_and_split_inputs_match_rails()')
 container='rust/crates/campfire/src/controllers/message_features/container_input_tests.rs'
 replace(container,'#[tokio::test]\nasync fn exceptional_root_and_nested_containers_match_rails_with_flat_reads()', 'async fn exceptional_root_and_nested_containers_match_rails_with_flat_reads()')
 replace(container,'#[tokio::test]\nasync fn exceptional_relative_consumers_match_rails_complete_state_with_flat_reads()', 'async fn exceptional_relative_consumers_match_rails_complete_state_with_flat_reads()')
 witnesses=[(1,relative,'relative/split actual output differs from Rails'),(2,relative,'relative/split actual output differs from Rails'),(3,container,'container actual HTTP envelope differs from Rails'),(4,container,'container persisted rows.status'),(5,container,'container actual HTTP envelope differs from Rails'),(6,container,'container actual publications UTC/slash_0'),(8,container,'container actual HTTP envelope differs from Rails')]
 for n,path,_ in witnesses:
  with (ROOT/path).open('a') as f:
   async_test=path==container
   f.write(f'\n#[{"tokio::test" if async_test else "test"}] {"async " if async_test else ""}fn ws8_input_producer_mutant_{n}() {{\n')
   f.write(f'campfire_db::slash_commands::time_parser::WS8_INPUT_MUTANT.store({n},std::sync::atomic::Ordering::SeqCst);\n')
   f.write((('exceptional_relative_consumers_match_rails_complete_state_with_flat_reads().await;' if n>=5 else 'exceptional_root_and_nested_containers_match_rails_with_flat_reads().await;') if async_test else 'exceptional_relative_and_split_inputs_match_rails();')+'\n}\n')
 for n,_,witness in witnesses:
  r=subprocess.run(runner+['test','--locked','-p','campfire','--bin','campfire',f'ws8_input_producer_mutant_{n}','-j2','--','--test-threads=4','--nocapture'],cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  print(r.stdout,flush=True)
  assert r.returncode!=0 and re.search(r"panicked at [^\n]+\n(?:assertion `left == right` failed: )?"+re.escape(witness),r.stdout),witness
  print(f'WS8bm2 input producer mutant {n}: rejected at {witness}',flush=True)
finally:
 for p,raw in originals.items():p.write_bytes(raw)
