#!/usr/bin/env python3
"""Mutate real work producers in isolated inputs, never tracked app source.

Starts with a normal control run that constructs all required inputs. The
reviewer's producer bodies run in fresh servers and databases. Independent
readback checks the app's rows, even when the normal driver stops at the browser.
"""
import argparse
import json
import os
import re
from pathlib import Path
import shutil
import subprocess
import sys

TOOLS = Path(__file__).resolve().parent
CASES = {
    'missing-history': 'converts a thread to work, assigns an eligible owner, and keeps an audit trail',
    'allow-reassignment': 'assigned owner can change work status but cannot reassign it',
    'missing-agent-events': 'a manager can assign an eligible agent and the agent is notified',
    'extra-foreign-event': 'converts a thread to work, assigns an eligible owner, and keeps an audit trail',
    'rewrite-history-client-id': 'converts a thread to work, assigns an eligible owner, and keeps an audit trail',
    'wrong-event-type': 'converts a thread to work, assigns an eligible owner, and keeps an audit trail',
    'wrong-event-actor': 'converts a thread to work, assigns an eligible owner, and keeps an audit trail',
    'displaced-history-identity': 'converts a thread to work, assigns an eligible owner, and keeps an audit trail',
}


def mutated_work(original):
    replacements = [
        ('WorkThreadEvent::create_for_change(tx, &before, &fresh, Some(actor), None)?;',
         'if !ws8bm_producer("missing-history") { WorkThreadEvent::create_for_change(tx, &before, &fresh, Some(actor), None)?; }'),
        ('''                crate::models::agent_work_events::record_owner_change(
                    tx,
                    &fresh,
                    before.work_owner_id,
                    owner,
                    Some(actor.id),
                )?;''', '''                if !ws8bm_producer("missing-agent-events") {
                    crate::models::agent_work_events::record_owner_change(tx, &fresh, before.work_owner_id, owner, Some(actor.id))?;
                }'''),
        ('    pub fn work_assignment_manageable_by(&self, conn: &Connection, user: &User) -> Result<bool> {\n',
         '    pub fn work_assignment_manageable_by(&self, conn: &Connection, user: &User) -> Result<bool> {\n        if ws8bm_producer("allow-reassignment") { return Ok(true); }\n'),
        ('''            Ok(())
        });
        *self = fresh;''', '''            if ws8bm_producer("rewrite-history-client-id") {
                tx.conn().execute("UPDATE messages SET client_message_id='corrupted-work-history' WHERE thread_id=?1 AND markdown_source='Keep this history'", [id])?;
            }
            if ws8bm_producer("extra-foreign-event") {
                tx.conn().execute("INSERT INTO work_thread_events (actor_id,channel_thread_id,created_at,event_type,from_status,to_status,updated_at) SELECT ?1,id,CURRENT_TIMESTAMP,'work_update','planned','in_progress',CURRENT_TIMESTAMP FROM channel_threads WHERE name='Revoked work owner'", [actor.id])?;
            }
            if ws8bm_producer("wrong-event-type") {
                tx.conn().execute("UPDATE work_thread_events SET event_type='work_update' WHERE channel_thread_id=?1 AND event_type='work_assignment'", [id])?;
            }
            if ws8bm_producer("wrong-event-actor") {
                tx.conn().execute("UPDATE work_thread_events SET actor_id=712064548 WHERE channel_thread_id=?1", [id])?;
            }
            if ws8bm_producer("displaced-history-identity") {
                let displaced = tx.conn().execute("UPDATE messages SET client_message_id='displaced-original-history' WHERE thread_id=?1 AND client_message_id='work-history' AND NOT EXISTS (SELECT 1 FROM messages WHERE thread_id=?1 AND client_message_id='displaced-original-history')", [id])?;
                if displaced == 1 {
                    tx.conn().execute("INSERT INTO messages (client_message_id,room_id,thread_id,creator_id,markdown_source,created_at,updated_at) SELECT 'work-history',room_id,thread_id,creator_id,markdown_source,created_at,updated_at FROM messages WHERE thread_id=?1 AND client_message_id='displaced-original-history'", [id])?;
                }
            }
            Ok(())
        });
        *self = fresh;'''),
    ]
    result = original
    for needle, replacement in replacements:
        assert result.count(needle) == 1, 'producer source anchor changed: ' + needle
        result = result.replace(needle, replacement)
    return result + '\nfn ws8bm_producer(mode: &str) -> bool { std::env::var("WS8BM_WORK_PRODUCER").ok().as_deref() == Some(mode) }\n'


READBACK = '''                finally:
                    try:
                        if file == "channel_threads_controller" and 'metadata' in locals():
                            from behavior_work_diagnostics import work_readback
                            work_readback(work, fixture, metadata, case, env)
                    finally:
                        stop_behavior_server('''



def driver_source(root, output):
    source = root / 'reference-tools/messaging/behavior-check.py'
    text = source.read_text()
    # Bootstrap above actually ran these unchanged setup commands in this invocation.
    starts = ('subprocess.run([str(ROOT / "parity/bin/frozen-seeds")', 'subprocess.run(["npm",')
    text = '\n'.join('pass # inputs built by this invocation\n' if line.startswith(starts) else line for line in text.splitlines())
    text = text.replace('str(target / "debug/campfire")', 'str(ROOT / ".scratch/ws8bm-work-producers/producer-campfire")')
    needle = '                finally:\n                    stop_behavior_server('
    assert text.count(needle) == 1
    text = text.replace(needle, READBACK)
    body = output / 'driver-body.py'
    body.write_text(text)
    wrapper = output / 'driver.py'
    wrapper.write_text('import sys\nfrom pathlib import Path\n'
                       + f'source=Path({str(source)!r})\n'
                       + 'sys.path.insert(0,str(source.parent))\n'
                       + f'exec(compile(Path({str(body)!r}).read_text(),str(source),"exec"),{{"__file__":str(source),"__name__":"__main__"}})\n')
    return wrapper


def intended_failure(text, marker):
    # Stop before the independent row receipts: a row error must never
    # provide attribution for the application's earlier browser failure.
    return any(marker in chunk for chunk in re.findall(
        r'WS8bm positive application FAILED:.*?(?=WS8bm failed application:|WS8bm positive application FAILED:|WS8bm browser flow FAILED:|WS8bm real producer rows:|\Z)',
        text,re.S))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=TOOLS.parents[1])
    parser.add_argument('--mutant', action='append', choices=CASES)
    parser.add_argument('--expect-escapes', action='store_true', help='baseline proof for the two reviewed escaping producers only')
    args = parser.parse_args()
    root = args.root.resolve()
    output = root / '.scratch/ws8bm-work-producers'
    output.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, CARGO_BUILD_JOBS='2', RUST_TEST_THREADS='8',
               TMPDIR=str(root / '.scratch'))
    target = Path(env.get('CARGO_TARGET_DIR', root / 'target')).resolve()
    env['CARGO_TARGET_DIR'] = str(target)
    bootstrap = ['python3', str(root / 'reference-tools/messaging/behavior-check.py'),
                 'channel_threads_controller', '--case', CASES['missing-history'], '--keep-going']
    with (output / 'bootstrap.log').open('w') as log:
        control = subprocess.run(bootstrap, cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT)
    assert control.returncode == 0, 'normal producer control failed; see ' + str(output / 'bootstrap.log')
    sys.path.insert(0, str(root / 'reference-tools/messaging'))
    from browser_host import prepare_source
    generated = prepare_source(root)
    source = generated / 'crates/db/src/models/channel_thread/work.rs'
    original = source.read_text()
    try:
        source.write_text(mutated_work(original))
        with (output / 'build.log').open('w') as log:
            build = subprocess.run(['cargo','build','--locked',
                                    '--manifest-path',str(generated / 'Cargo.toml'),'-p','campfire','--bin','campfire'],
                                   cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT)
        assert build.returncode == 0, 'producer build failed; see ' + str(output / 'build.log')
        shutil.copyfile(target / 'debug/campfire', output / 'producer-campfire')
        (output / 'producer-campfire').chmod(0o755)
    finally:
        source.write_text(original)
    driver = driver_source(root, output)
    modes = args.mutant or list(CASES)
    failed = 0
    for mode in modes:
        case = CASES[mode]
        with (output / f'{mode}.log').open('w') as log:
            result = subprocess.run(['python3',str(driver),'channel_threads_controller','--case',case,'--keep-going'],
                                    cwd=root, env=dict(env,WS8BM_WORK_PRODUCER=mode),stdout=log,stderr=subprocess.STDOUT)
        text = (output / f'{mode}.log').read_text()
        rows = [json.loads(line.removeprefix('WS8bm real producer rows: ')) for line in text.splitlines() if line.startswith('WS8bm real producer rows: ')]
        paired=(len(rows)==1 and rows[0]['app']=='Rust' and rows[0].get('row_assertion')!='INVALID')
        escape = args.expect_escapes and mode in {'extra-foreign-event','rewrite-history-client-id'}
        if not paired:
            valid=False
        elif escape:
            valid = result.returncode == 0 and all(row['row_assertion']=='PASS' for row in rows)
            valid &= (all(row['global_event_delta']==4 and row['target_event_delta']==2 for row in rows)
                      if mode=='extra-foreign-event' else all(row['history_client_id']=='corrupted-work-history' for row in rows))
        elif mode == 'extra-foreign-event':
            valid = (result.returncode != 0 and intended_failure(text,'work-event-count:')
                     and all(row['global_event_delta']==2 and row['target_event_delta']==1 for row in rows))
        elif mode == 'rewrite-history-client-id':
            valid = (result.returncode != 0 and all(row['history_client_id']=='corrupted-work-history'
                     and row['row_assertion']=='FAIL' and 'work-history identity:' in row.get('row_error','') for row in rows))
        elif mode == 'displaced-history-identity':
            valid = (result.returncode != 0 and all(row['history_client_id']=='displaced-original-history'
                     and row['history_lookup'] is not None and row['history_lookup'][0]!=row['original_history_id']
                     and row['history_lookup'][1]=='Keep this history' and row['row_assertion']=='FAIL'
                     and 'work-history identity:' in row.get('row_error','') for row in rows))
        elif mode in {'wrong-event-type','wrong-event-actor'}:
            markers=("'work_update'","'work_assignment'") if mode=='wrong-event-type' else ('712064548','773523953')
            index,expected=(1,'work_update') if mode=='wrong-event-type' else (0,712064548)
            valid=(result.returncode!=0 and all(intended_failure(text,marker) for marker in markers)
                   and all(row['events'] and row['events'][-1][index]==expected
                           and row['row_assertion']=='FAIL' for row in rows))
        elif mode == 'missing-history':
            valid = (result.returncode != 0 and intended_failure(text,'work-event-count:')
                     and all(row['events']==[] and row['thread'][0]=='planned' for row in rows))
        elif mode == 'allow-reassignment':
            valid = result.returncode != 0 and intended_failure(text,'200 !== 403') and all(row['thread'][1]==773523953 for row in rows)
        else:
            valid = (result.returncode != 0 and intended_failure(text,'agent-event-count:')
                     and all(row['agent_events']==0 and row['row_assertion']=='FAIL'
                             and 'agent-event-count:' in row.get('row_error','') for row in rows))
        failed += not valid
        print(f'WS8bm real producer discrimination: {mode}: ' + ('ESCAPED as expected at baseline' if escape and valid else 'REJECTED at intended assertion' if valid else 'INVALID or unexpected result'),flush=True)
        for row in rows:
            print(json.dumps(row),flush=True)
    print(f'WS8bm producer discrimination check: {len(modes)-failed} proofs; {failed} invalid or unexpected; '+('baseline escapes expected' if args.expect_escapes else 'no escapes accepted'),flush=True)
    return bool(failed)


if __name__ == '__main__':
    sys.exit(main())
