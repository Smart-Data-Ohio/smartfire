#!/usr/bin/env python3
"""Fail closed on the remaining originals' maps and current paired executions."""
import argparse,hashlib,json,re,subprocess
from collections import Counter
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--nextest-list',type=Path,required=True)
p.add_argument('--browser-log',type=Path,required=True)
p.add_argument('--manifest',type=Path,action='append',required=True)
p.add_argument('--mutations',type=Path)
a=p.parse_args();root=Path(__file__).resolve().parents[2]
pin=(root/'rust/parity/reference.sha').read_text().strip()
listing=json.loads(a.nextest_list.read_text())
tests={n:v for s in listing['rust-suites'].values() for n,v in s['testcases'].items()}
log=a.browser_log.read_text();passed=set(re.findall(r'^\s*PASS\s+\[[^\]]+\]\s+\([^)]+\)\s+\S+\s+(\S+)',log,re.M))
source={}
def oracle(path):
 if path not in source:source[path]=subprocess.check_output(['git','show',pin+':'+path],cwd=root,text=True)
 return source[path]
def anchor(check):
 path,n=check['path'].rsplit(':',1)
 assert (root/path).read_text().splitlines()[int(n)-1].strip()==check['anchor'],('stale assertion anchor',check)
sections={}
for hit in re.finditer(r'^\s*Original (Rails|Rust) ([\w-]+):\n',log,re.M):
 next_hit=re.search(r'^\s*Original (?:Rails|Rust) [\w-]+:\n',log[hit.end():],re.M)
 end=hit.end()+next_hit.start() if next_hit else len(log)
 sections[hit[1],hit[2]]=log[hit.end():end]
records=[];sites=helpers=interactions=prerequisites=0;legacy=additional=0
for path in a.manifest:
 manifest=json.loads(path.read_text());assert manifest['reference']==pin
 for implementation in manifest.get('browser_dependencies',[]):
  assert hashlib.sha256((root/implementation['path']).read_bytes()).hexdigest()==implementation['sha256'],('browser dependency changed after receipt',implementation['path'])
 if 'fixture_foundation_sha256' in manifest:
  assert hashlib.sha256((root/manifest['fixture_foundation']).read_bytes()).hexdigest()==manifest['fixture_foundation_sha256'],('original fixture foundation changed after receipt',path)
 for implementation,digest in manifest.get('implementation_sha256',{}).items():
  assert hashlib.sha256((root/implementation).read_bytes()).hexdigest()==digest,('implementation changed after receipt',implementation)
 legacy+=len(manifest['records']);additional+=len(manifest.get('current_pin_records',[]))
 for r in manifest['records']+manifest.get('current_pin_records',[]):
  records.append(r);src=oracle(r['file'])
  assert hashlib.sha256(src.encode()).hexdigest()==r['source_sha256'],r['id']
  declarations=list(re.finditer(r'^  test "([^"]+)" do$',src,re.M));decl=next(x for x in declarations if x[1]==r['test']);i=declarations.index(decl)
  end=declarations[i+1].start() if i+1<len(declarations) else src.find('\n  private',decl.end());end=len(src) if end<0 else end
  start=src[:decl.start()].count('\n')+1
  direct={start+i:s.strip() for i,s in enumerate(src[decl.start():end].splitlines()) if s.strip().startswith(('assert','refute')) or (s.strip().startswith('within') and re.search(r'\b(?:assert(?:_\w+)?|refute(?:_\w+)?)\b',s))}
  assert direct=={x['line']:x['ruby'] for x in r['assertions']},('unaccounted original assertion',r['id'],direct)
  for t in r['rust_tests']:assert tests[t]['ignored'] and t in passed,('wrapper missing successful explicit execution',t)
  for x in r['assertions']:
   anchor({'path':x['assertion_source'],'anchor':x['assertion_anchor']})
   for q in x.get('additional_assertion_sources',[]):anchor(q)
  sites+=len(r['assertions'])
  for x in r.get('helper_expansion',[])+r.get('setup_expansion',[]):
   file=x.get('source_file',x.get('ruby_file',r['file']))
   assert oracle(file).splitlines()[x['line']-1].strip()==x['ruby'],('stale helper',r['id'],x)
   anchor(x['rust_assertion']);helpers+=1
  for x in r.get('interaction_helper_sources',[]):
   file=x.get('source_file',x.get('ruby_file',r['file']))
   assert oracle(file).splitlines()[x['line']-1].strip()==x['ruby'],('stale prerequisite helper',x)
   anchor(x['rust_helper']);interactions+=1
  for key in ['fixture_setup_sources','environment_setup_sources','environment_restoration_sources','authentication_sources']:
   for x in r.get(key,[]):
    file=x.get('source_file',x.get('ruby_file',r['file']))
    assert oracle(file).splitlines()[x['line']-1].strip()==x['ruby'],('stale original prerequisite',r['id'],x)
    anchor(x['implementation']);prerequisites+=1
  for target in ['Rails','Rust']:
   section=sections[target,r['browser_case']]
   assert f"ORIGINAL_CASE {r['browser_case']}: passed" in section,('missing full paired original',target,r['id'])
   # Every direct assertion (including scoped inline sites and helper calls)
   # requires its own successful receipt; all helper expansion sites do too.
   for x in r['assertions']:
    assert f"ORIGINAL_ASSERTION {r['file']}:{x['line']}" in section,('unexecuted mapped assertion',target,r['id'],x['line'])
   for x in r.get('helper_expansion',[])+r.get('setup_expansion',[]):
    file=x.get('source_file',x.get('ruby_file',r['file']))
    assert f"ORIGINAL_ASSERTION {file}:{x['line']}" in section,('unexecuted helper',target,r['id'],file,x['line'])
   expected=Counter(f"{r['file']}:{x['line']}" for x in r['assertions'])
   expected.update(f"{x.get('source_file',x.get('ruby_file',r['file']))}:{x['line']}" for x in r.get('helper_expansion',[])+r.get('setup_expansion',[]))
   observed=Counter(re.findall(r'ORIGINAL_ASSERTION ([^\s]+)',section))
   assert all(observed[k]>=n for k,n in expected.items()),('missing repeated original assertion invocation',target,r['id'],expected-observed)
if a.mutations:
 m=json.loads(a.mutations.read_text());assert m['reference']==pin;valid=m['mutations']
 combined=[]
 for receipt in m.get('source_receipts',[]):
  path=root/receipt['path'];assert hashlib.sha256(path.read_bytes()).hexdigest()==receipt['sha256'],('mutation source receipt drift',receipt)
  source_receipt=json.loads(path.read_text());assert source_receipt['reference']==pin
  if 'fixture_foundation_sha256' in source_receipt:
   assert hashlib.sha256((root/source_receipt['fixture_foundation']).read_bytes()).hexdigest()==source_receipt['fixture_foundation_sha256'],('mutation fixture foundation drift',receipt)
  combined.extend(source_receipt['mutations'])
 if m.get('source_receipts'):assert combined==valid, 'only current source controls may receive aggregate credit'
 assert all(x['producer_restored'] and x['intended_assertion_failed'] and not x.get('invalid_control',False) for x in valid)
 by_id={r['id']:r for r in records}
 for x in valid:
  assert x['closure_id'] in by_id,('mutation credits an unknown closure',x)
  r=by_id[x['closure_id']]
  coordinates={f"{r['file']}:{v['line']}" for v in r['assertions']}
  for v in r.get('helper_expansion',[])+r.get('setup_expansion',[])+r.get('interaction_helper_sources',[]):
   coordinates.add(f"{v.get('source_file',v.get('ruby_file',r['file']))}:{v['line']}")
  assert x['assertion'] in coordinates,('mutation targets an unmapped assertion',x)
  assert 'ORIGINAL_PRODUCER_RESTORED' in x.get('restoration_output','')+x.get('failure_output',''),('producer restoration not evidenced',x)
 unique={x['closure_id'] for x in valid};assert len(unique)>=30,('insufficient distinct closure discrimination',len(unique))
 for x in valid:
  if 'failure_output' in x:
   assert hashlib.sha256(x['failure_output'].encode()).hexdigest()==x.get('evidence_sha256',x.get('failure_output_sha256')),('mutation evidence drift',x)
   assert x['assertion'] in x['failure_output'],('mutation failed elsewhere',x)
   assert re.search(r'(?m)^(?:[A-Za-z]*Error|AssertionError)[^\n]*'+re.escape(x['assertion']),x['failure_output']),('mutation target appears only in a successful marker',x)
  else:
   evidence=root/x['evidence_path'];assert hashlib.sha256(evidence.read_bytes()).hexdigest()==x.get('evidence_sha256',x.get('failure_output_sha256')),('mutation evidence drift',x)
 print(f'D producer mutations: {len(valid)} valid controls; {len(unique)} distinct closures; 0 invalid controls credited')
print(f'D original assertion receipts: {legacy} ledger declarations + {additional} current-pin additions; {sites} direct sites; {helpers} helper/setup expansions; {interactions} synchronized premises; {prerequisites} fixture/environment/authentication premises; {2*len(records)} paired executions; 0 unaccounted assertions')
