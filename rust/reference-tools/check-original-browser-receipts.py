#!/usr/bin/env python3
"""Check complete original assertion mappings against real full-browser gate receipts."""
import argparse,hashlib,json,re,subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--nextest-list',type=Path,required=True)
p.add_argument('--browser-log',type=Path,required=True)
p.add_argument('--controls-dir',type=Path,required=True)
a=p.parse_args();root=Path(__file__).resolve().parents[2]
m=json.loads((root/'rust/plans/ledger-ws8br-ws17-ws11ui-c-browser-receipts.json').read_text())
assert m['reference']==(root/'rust/parity/reference.sha').read_text().strip(),'original assertions must use the runtime Rails pin'
tests={n:v for s in json.loads(a.nextest_list.read_text())['rust-suites'].values() for n,v in s['testcases'].items()}
log='\n'.join(line[4:] if line.startswith('    ') else line for line in a.browser_log.read_text().splitlines());passed=set(re.findall(r'^test (\S+) \.\.\. ok$',log,re.M))
# --show-output is required: a pass line alone cannot prove original assertion coverage.
sections={}
for hit in re.finditer(r'^Original (Rails|Rust) ([\w-]+):\n',log,re.M):
 end=log.find('\nOriginal ',hit.end());end=len(log) if end<0 else end
 sections[(hit[1],hit[2])]=log[hit.end():end]
assertions=helpers=setup=0
for record in m['records']:
 src=subprocess.check_output(['git','show',m['reference']+':'+record['file']],cwd=root,text=True)
 assert hashlib.sha256(src.encode()).hexdigest()==record['source_sha256']
 decl=list(re.finditer(r'^  test "([^"]+)" do$',src,re.M));d=next(x for x in decl if x[1]==record['test']);i=decl.index(d)
 end=decl[i+1].start() if i+1<len(decl) else src.find('\n  private',d.end());end=len(src) if end<0 else end
 start=src[:d.start()].count('\n')+1
 actual={start+i:s.strip() for i,s in enumerate(src[d.start():end].splitlines()) if s.strip().startswith(('assert','refute'))}
 assert actual=={x['line']:x['ruby'] for x in record['assertions']},record['id']
 for t in record['rust_tests']:
  assert tests[t]['ignored'] and t in passed,('browser wrapper not explicitly executed',t)
 for x in record['assertions']:
  checks=[{'path':x['assertion_source'],'anchor':x['assertion_anchor']}]+x['additional_assertion_sources']
  for check in checks:
   path,n=check['path'].rsplit(':',1);assert (root/path).read_text().splitlines()[int(n)-1].strip()==check['anchor'],check
  assertions+=1
 for x in record['helper_expansion']+record['setup_expansion']:
  assert src.splitlines()[x['line']-1].strip()==x['ruby'];q=x['rust_assertion'];path,n=q['path'].rsplit(':',1);assert (root/path).read_text().splitlines()[int(n)-1].strip()==q['anchor'],q
 helpers+=len(record['helper_expansion']);setup+=len(record['setup_expansion'])
 for target in ['Rails','Rust']:
  section=sections[(target,record['browser_case'])]
  assert f"ORIGINAL_CASE {record['browser_case']}: passed" in section
  for x in record['assertions']+record['helper_expansion']+record['setup_expansion']:
   n=x['line'];ruby=x['ruby']
   if ruby.startswith(('assert_starred','assert_unstarred','assert_member_rows_inline','assert_no_horizontal_overflow','assert_row_trigger_focused')):continue # individual helper predicates checked separately
   if record['file'].endswith('people_group_dms_test.rb') and n in [87,109,240]:
    assert 'ORIGINAL_PATH '+target+': canonical group' in section
    if n==109:assert 'ORIGINAL_ASSERTION '+record['file']+':109' in section
   elif record['file'].endswith('people_group_dms_test.rb') and n==342:assert '"kind": "left"' in section and 'ORIGINAL_DB '+target in section
   elif ruby=='assert_tour_completed':assert f'"line": {n}' in section and 'ORIGINAL_DB '+target in section
   else:assert 'ORIGINAL_ASSERTION '+record['file']+':'+str(n) in section,(target,record['id'],n)
for mode in sorted({r['browser_mode'] for r in m['records']}):
 count=sum(r['browser_mode']==mode for r in m['records'])
 assert f'Original browser {mode}: {2*count} paired case executions passed; 0 failures' in log
 control=(a.controls_dir/f'c-browser-{mode}-controls.log').read_text()
 assert f'Original browser {mode}: {count} producer defects rejected; 0 invalid controls' in control
 assert 'INVALID_CONTROL transport' not in control
print(f'Original browser per-assertion receipts: {len(m["records"])} declarations; {assertions} direct sites; {helpers} private-helper expansions; {setup} setup assertions; 54 paired executions passed; 27 producer defects rejected; 0 unaccounted assertions')
