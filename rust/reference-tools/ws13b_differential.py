#!/usr/bin/env python3
"""Reproducible observed Rails differential, including real Stimulus banner state.

record COUNT OUTPUT: run the pinned Rails generator and real banner controller.
check ORACLE OBSERVATIONS_DIR: compare all Rust observations, then replay both
through the actual controller. Tests write observations with
WS13B_OBSERVED_ORACLE and WS13B_DIFFERENTIAL_OUTPUT. No delivery projection.
shrink FAILURE OUTPUT: delta-debug a recorded mismatch, rerunning BOTH apps.
"""
import copy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
CARGO = ['mise','exec','rust@1.98.1','--','cargo','test','--locked','--manifest-path','rust/Cargo.toml','--workspace','--exclude','html5ever','observed_rails_differential_','--','--test-threads=8','--nocapture']
ENV = dict(os.environ,PARITY_NAMESPACE='ws13b',PARITY_IMAGE='ws13-reference:d7c7de92',PARITY_OWNER='ws13b',PARITY_CPUS='2',CARGO_BUILD_JOBS='2')

def banner(value, directory):
    with tempfile.NamedTemporaryFile(mode="w",suffix=".json",dir=directory) as source:
        source.write(json.dumps(value));source.flush()
        out = subprocess.check_output(['node','--no-warnings','--experimental-vm-modules','rust/reference-tools/ws13b_banner_observer.mjs',source.name],cwd=ROOT)
    return json.loads(out)

def record(count, output, specs=None):
    command = ['rust/parity/bin/reference','runner']
    if specs is not None:
        command += ['-e','WS13B_SPEC_JSON='+json.dumps(specs)]
    command += ['rust/reference-tools/ws13b_ring_matrix.rb',str(count)]
    result = subprocess.check_output(command,cwd=ROOT,env=ENV)
    observed = json.loads(result)
    observed = banner(observed,output.parent)
    output.write_text(json.dumps(observed,separators=(',',':'))+'\n')
    return observed

def verify_sources(oracle):
    assert oracle['reference_pin']=='d7c7de92'
    for path,digest in oracle['source_sha256'].items():
        raw=subprocess.check_output(['git','show',f'd7c7de92:{path}'],cwd=ROOT)
        assert hashlib.sha256(raw).hexdigest()==digest,path

def compare(expected, actual):
    expected_by_name={c['spec']['name']:c for c in expected['cases']}
    actual_by_name={c['spec']['name']:c for c in actual['cases']}
    assert len(expected_by_name)==len(expected['cases'])
    assert len(actual_by_name)==len(actual['cases'])
    assert expected_by_name.keys()==actual_by_name.keys(),'case inventory differs'
    for name in expected_by_name:
        assert actual_by_name[name]==expected_by_name[name],f'observed sequence differs: {name}'

def check(oracle, directory):
    verify_sources(oracle)
    cases=[]
    for part in range(8):
        cases += json.loads((directory/f'observations-{part}.json').read_text())
    actual=banner({'cases':cases},directory)
    compare(oracle,actual)
    # The comparison itself must reject lost/duplicate frames, push and banner
    # corruption; these are observed outputs, never edited expected fixtures.
    rejected=[]
    for kind in ['lost_frame','duplicate_frame','metadata','push','banner']:
        changed=copy.deepcopy(actual)
        if kind=='banner':
            phase=changed['cases'][0]['phases'][0]
            client=next(iter(phase['banners'].values()))
            client['hidden']=not client['hidden']
        elif kind=='push':
            phase=next(p for c in changed['cases'] for p in c['phases'] if p['pushes'])
            phase['pushes'][0]['payload']['title']='CORRUPTED'
        else:
            phase=next(p for c in changed['cases'] for p in c['phases'] if p['frames'])
            if kind=='lost_frame':phase['frames'].pop()
            elif kind=='duplicate_frame':phase['frames'].append(copy.deepcopy(phase['frames'][0]))
            else:phase['frames'][0]['payload']['huddleInvitation']['callerName']='CORRUPTED'
        try:compare(oracle,changed)
        except AssertionError:rejected.append(kind)
        else:raise AssertionError('comparator accepted '+kind)
    count=len(oracle['cases']);random=sum(c['spec']['name'].startswith('random/') for c in oracle['cases'])
    phases=sum(len(c['phases']) for c in oracle['cases'])
    print(f'Observed Rails differential: {count}/{count} sequences; {random} random; {phases} steps; broadcasts, push handoffs, items and real banner states match')
    print('Seeds: '+', '.join(str(s) for s in oracle['seeds']))
    print(f'Differential discrimination: {len(rejected)}/5 output corruptions rejected')

def shrink(failure, output):
    spec=failure['spec']
    original=len(spec['steps'])
    directory=output.parent
    executable=os.environ.get("WS13B_SHRINK_TEST_BIN")
    if not executable:
        build=CARGO[:-4]+['--no-run','--message-format=json']
        artifacts=subprocess.check_output(build,cwd=ROOT,env=ENV,text=True)
        executable=next(row['executable'] for row in map(json.loads,artifacts.splitlines()) if row.get('reason')=='compiler-artifact' and row['target']['name']=='campfire_db' and row.get('executable'))
    rails_log=(directory/'shrink-rails.log').open('w')
    stream=subprocess.Popen(['rust/parity/bin/reference','runner','-e','WS13B_STREAM=1','rust/reference-tools/ws13b_ring_matrix.rb','0'],cwd=ROOT,env=ENV,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=rails_log,text=True)
    def mismatch(steps):
        candidate=copy.deepcopy(spec);candidate['steps']=steps
        for step in steps:step.pop('job_order',None)
        stream.stdin.write(json.dumps([candidate])+"\n");stream.stdin.flush()
        line=stream.stdout.readline()
        assert line,"Rails observation stream stopped unexpectedly"
        oracle=json.loads(line)
        if 'error' in oracle:return False
        (directory/'shrink-oracle.json').write_text(json.dumps(oracle))
        env=dict(ENV,CI='1',WS13B_OBSERVED_ORACLE=str(directory/'shrink-oracle.json'),WS13B_DIFFERENTIAL_OUTPUT=str(directory),TMPDIR=str(ROOT/'.scratch'))
        with (directory/'shrink-rust.log').open('w') as log:
            result=subprocess.run([executable,"observed_rails_differential_","--test-threads=8","--nocapture"],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT)
        return result.returncode!=0 and any(json.loads((directory/f'mismatches-{part}.json').read_text()) for part in range(8))
    steps=copy.deepcopy(spec['steps']);assert mismatch(steps),"original mismatch did not reproduce"
    size=max(1,(len(steps)-1)//2)
    while size:
        reduced=False
        for start in range(1,len(steps),size):
            candidate=steps[:start]+steps[start+size:]
            if mismatch(candidate):steps=candidate;reduced=True;break
        if not reduced:size//=2
    # One-operation irreducible under deletion; also minimize clock advances.
    for index in range(1,len(steps)):
        for seconds in [0,1,121,181]:
            if seconds>=steps[index]['seconds']:continue
            candidate=copy.deepcopy(steps);candidate[index]['seconds']=seconds
            if mismatch(candidate):steps=candidate;break
    stream.stdin.close();stream.wait();rails_log.close()
    spec['steps']=steps;output.write_text(json.dumps(spec,indent=2)+'\n')
    print(f'Shrunk {spec["name"]}: {original} -> {len(steps)} operations; replay spec: {output}')

if __name__=='__main__':
    action=sys.argv[1]
    if action=='record':record(int(sys.argv[2]),Path(sys.argv[3]).resolve())
    elif action=='check':check(json.loads(Path(sys.argv[2]).read_text()),Path(sys.argv[3]).resolve())
    elif action=='shrink':
        failures=json.loads(Path(sys.argv[2]).read_text());failure=failures[int(sys.argv[4])] if len(sys.argv)>4 else failures[0]
        shrink(failure,Path(sys.argv[3]).resolve())
    else:raise SystemExit(__doc__)
