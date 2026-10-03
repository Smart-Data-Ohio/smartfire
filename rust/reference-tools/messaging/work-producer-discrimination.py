#!/usr/bin/env python3
"""Mutate real work producers in isolated inputs, never tracked app source.

Starts with a normal paired control that constructs all required inputs. The
reviewer's producer bodies run in fresh servers and databases. Independent
readback checks both applications, even when the normal driver stops at Rails.
"""
import argparse
import json
import os
import re
from pathlib import Path
import shutil
import shlex
import subprocess
import sys

TOOLS = Path(__file__).resolve().parent
CASES = {
    'missing-history': 'converts a thread to work, assigns an eligible owner, and keeps an audit trail',
    'allow-reassignment': 'assigned owner can change work status but cannot reassign it',
    'missing-agent-events': 'a manager can assign an eligible agent and the agent is notified',
    'extra-foreign-event': 'converts a thread to work, assigns an eligible owner, and keeps an audit trail',
    'rewrite-history-client-id': 'converts a thread to work, assigns an eligible owner, and keeps an audit trail',
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
                    if file == "channel_threads_controller" and 'metadata' in locals():
                        from behavior_work_rows import assert_work_rows
                        for app, db in [("Rails", work / f".instances/{ports[0]}/db/production.sqlite3"), ("Rust", work / "db/production.sqlite3")]:
                            with sqlite3.connect(f"file:{db}?mode=ro", uri=True) as candidate, sqlite3.connect(f"file:{fixture / 'db/production.sqlite3'}?mode=ro", uri=True) as original:
                                state={"app":app,"producer":env['WS8BM_WORK_PRODUCER']}
                                state['thread']=candidate.execute('SELECT work_status,work_owner_id FROM channel_threads WHERE id=?',(metadata['thread_id'],)).fetchone()
                                state['events']=candidate.execute('SELECT actor_id,event_type,from_status,to_status,from_owner_id,to_owner_id FROM work_thread_events WHERE channel_thread_id=? ORDER BY id',(metadata['thread_id'],)).fetchall()
                                state['agent_events']=candidate.execute("SELECT COUNT(*) FROM agent_events WHERE agent_id=? AND event_type='work_assigned'",(metadata['eligible_agent_id'],)).fetchone()[0]
                                state['history_client_id']=candidate.execute('SELECT client_message_id FROM messages WHERE id=?',(metadata['history_message_id'],)).fetchone()[0]
                                state['global_event_delta']=candidate.execute('SELECT COUNT(*) FROM work_thread_events').fetchone()[0]-original.execute('SELECT COUNT(*) FROM work_thread_events').fetchone()[0]
                                state['target_event_delta']=len(state['events'])-original.execute('SELECT COUNT(*) FROM work_thread_events WHERE channel_thread_id=?',(metadata['thread_id'],)).fetchone()[0]
                                try:
                                    assert_work_rows(candidate,original,case,metadata)
                                    state['row_assertion']='PASS'
                                except AssertionError as error:
                                    state['row_assertion']='FAIL';state['row_error']=str(error)
                                print('WS8bm real producer rows: '+json.dumps(state),flush=True)
                    if process is not None:'''


def driver_source(root, output):
    source = root / 'rust/reference-tools/messaging/behavior-check.py'
    text = source.read_text()
    # Bootstrap above actually ran these unchanged setup commands in this invocation.
    starts = ('subprocess.run(["bash", "rust/parity/bin/seed"',
              'subprocess.run(["mise", "exec", "rust@1.98.1"',
              'subprocess.run(["npm",', 'subprocess.run(["docker", "build",')
    text = '\n'.join('pass # inputs built by this invocation\n' if line.startswith(starts) else line for line in text.splitlines())
    text = text.replace('reference_up=[reference,', 'reference_up=[str(ROOT / ".scratch/ws8bm-work-producers/reference-producer"),')
    text = text.replace('str(target / "debug/campfire")', 'str(ROOT / ".scratch/ws8bm-work-producers/producer-campfire")')
    needle = '                finally:\n                    if process is not None:'
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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=TOOLS.parents[2])
    parser.add_argument('--mutant', action='append', choices=CASES)
    parser.add_argument('--expect-escapes', action='store_true', help='baseline proof for the two reviewed escaping producers only')
    args = parser.parse_args()
    root = args.root.resolve()
    output = root / '.scratch/ws8bm-work-producers'
    output.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, CARGO_BUILD_JOBS='2', RUST_TEST_THREADS='8',
               CAMPFIRE_REFERENCE=str(root), TMPDIR=str(root / '.scratch'))
    target = Path(env.get('CARGO_TARGET_DIR', root / 'rust/target')).resolve()
    env['CARGO_TARGET_DIR'] = str(target)
    bootstrap = ['python3', str(root / 'rust/reference-tools/messaging/behavior-check.py'),
                 'channel_threads_controller', '--case', CASES['missing-history'], '--keep-going']
    with (output / 'bootstrap.log').open('w') as log:
        control = subprocess.run(bootstrap, cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT)
    assert control.returncode == 0, 'normal producer control failed; see ' + str(output / 'bootstrap.log')
    sys.path.insert(0, str(root / 'rust/reference-tools/messaging'))
    from browser_host import prepare_source
    generated = prepare_source(root)
    source = generated / 'crates/db/src/models/channel_thread/work.rs'
    original = source.read_text()
    try:
        source.write_text(mutated_work(original))
        with (output / 'build.log').open('w') as log:
            build = subprocess.run(['mise','exec','rust@1.98.1','--','cargo','build','--locked',
                                    '--manifest-path',str(generated / 'Cargo.toml'),'-p','campfire','--bin','campfire'],
                                   cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT)
        assert build.returncode == 0, 'producer build failed; see ' + str(output / 'build.log')
        shutil.copyfile(target / 'debug/campfire', output / 'producer-campfire')
        (output / 'producer-campfire').chmod(0o755)
    finally:
        source.write_text(original)
    ruby = output / 'producer.rb'
    shutil.copyfile(TOOLS / 'work-producer-mutants.rb', ruby)
    reference = (root / 'rust/parity/bin/reference').read_text()
    root_lines = [line for line in reference.splitlines() if line.startswith('ROOT=')]
    assert len(root_lines) == 1
    reference = reference.replace(root_lines[0], 'ROOT=' + shlex.quote(str(root / 'rust')))
    needle = '  printf \'%s\\n\' --env-file "$ENV_FILE"'
    assert reference.count(needle) == 1
    reference = reference.replace(needle, needle + f'\n  printf \'%s\\n\' -v {ruby}:/rails/config/initializers/ws8bm_work_producer.rb:ro -e "WS8BM_WORK_PRODUCER=${{WS8BM_WORK_PRODUCER:?}}"')
    (output / 'reference-producer').write_text(reference)
    (output / 'reference-producer').chmod(0o755)
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
        assert len(rows) == 2, mode + ': missing independent paired readback'
        def intended_on_both(marker):
            return all(any(marker in chunk for chunk in re.findall(
                rf'WS8bm positive application FAILED: {app}:.*?(?=WS8bm failed application:|WS8bm positive application FAILED:|\Z)',
                text,re.S)) for app in ['Rails','Rust'])
        escape = args.expect_escapes and mode in {'extra-foreign-event','rewrite-history-client-id'}
        if escape:
            valid = result.returncode == 0 and all(row['row_assertion']=='PASS' for row in rows)
            valid &= (all(row['global_event_delta']==4 and row['target_event_delta']==2 for row in rows)
                      if mode=='extra-foreign-event' else all(row['history_client_id']=='corrupted-work-history' for row in rows))
        elif mode == 'extra-foreign-event':
            valid = (result.returncode != 0 and intended_on_both('work-event-count:')
                     and all(row['global_event_delta']==2 and row['target_event_delta']==1 for row in rows))
        elif mode == 'rewrite-history-client-id':
            valid = (result.returncode != 0 and all(row['history_client_id']=='corrupted-work-history'
                     and row['row_assertion']=='FAIL' and 'work-history identity:' in row.get('row_error','') for row in rows))
        elif mode == 'missing-history':
            valid = (result.returncode != 0 and intended_on_both('work-event-count:')
                     and all(row['events']==[] and row['thread'][0]=='planned' for row in rows))
        elif mode == 'allow-reassignment':
            valid = result.returncode != 0 and intended_on_both('200 !== 403') and all(row['thread'][1]==773523953 for row in rows)
        else:
            valid = (result.returncode != 0 and intended_on_both('agent-event-count:')
                     and all(row['agent_events']==0 and row['row_assertion']=='FAIL'
                             and 'agent-event-count:' in row.get('row_error','') for row in rows))
        failed += not valid
        print(f'WS8bm real producer discrimination: {mode}: ' + ('ESCAPED as expected at baseline' if escape and valid else 'REJECTED on Rails and Rust at intended assertion' if valid else 'INVALID or unexpected result'),flush=True)
        for row in rows:
            print(json.dumps(row),flush=True)
    print(f'WS8bm producer discrimination check: {len(modes)-failed} paired proofs; {failed} invalid or unexpected; '+('baseline escapes expected' if args.expect_escapes else 'no escapes accepted'),flush=True)
    return bool(failed)


if __name__ == '__main__':
    sys.exit(main())
