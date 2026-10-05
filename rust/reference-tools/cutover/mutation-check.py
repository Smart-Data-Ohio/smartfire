#!/usr/bin/env python3
"""Run temporary semantic production mutations, restoring bytes even on failure.

The command prefix after -- must run cargo in the canonical environment and pass
CAMPFIRE_LEDGER_MUTATION into it. Only a selected native test's assertion failure
counts; compilation/setup failures are not accepted as killed mutations.
"""
import argparse, hashlib, json, os, pathlib, re, shutil, subprocess, sys
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('manifest',type=pathlib.Path);p.add_argument('output',type=pathlib.Path)
p.add_argument('--recheck',action='store_true',help='Reparse existing raw runs; verify restored production hashes')
# Parse the command only after the explicit separator. argparse.REMAINDER
# previously consumed a trailing --recheck as a command and entered mutation.
argv = sys.argv[1:]
separator = argv.index('--') if '--' in argv else len(argv)
command = argv[separator + 1:]
a=p.parse_args(argv[:separator]);root=pathlib.Path(__file__).resolve().parents[3];os.chdir(root)
a.output.mkdir(parents=True,exist_ok=True)
def assertion_failure(s, expected=None):
 panic=re.findall(r"panicked at ([^\n]+)",s)
 for cited in panic:
  match=re.search(r'([^ :]+):(\d+):\d+:',cited)
  if not match: continue
  file,line=match[1],int(match[2]);path=root/'rust'/file
  if path.exists() and re.match(r'assert(?:_eq|_ne)?!\(',path.read_text().splitlines()[line-1].strip()):
   if expected:
    actual=str(path.resolve().relative_to(root))
    if not any(e['file']==actual and e['line']==line for e in expected):continue
   return panic,True
 return panic,False
if a.recheck:
 receipt=json.loads((a.output/'results.json').read_text())
 assert all(hashlib.sha256((root/p).read_bytes()).hexdigest()==h for p,h in receipt['source_hashes'].items())
 for r in receipt['results']:
  s=(a.output/r['log']).read_text();r['panic'],valid=assertion_failure(s,r.get('expected_assertions'))
  r['killed']=r['exit_code']!=0 and any('1 test run: 0 passed, 1 failed' in l for l in r['summary']) and valid
 (a.output/'results.json').write_text(json.dumps(receipt,indent=2)+'\n')
 print(f'Mutation summary: {len(receipt["results"])} run; {sum(r["killed"] for r in receipt["results"])} killed; production restored={receipt["production_restored"]}')
 sys.exit(0 if all(r['killed'] for r in receipt['results']) else 1)
assert command and not command[0].startswith('-'), 'Use -- followed by the executable command prefix'
assert shutil.which(command[0]), ('Command executable is unavailable', command[0])
mutations=json.loads(a.manifest.read_text())['mutations'];original={};results=[]
try:
 for m in mutations:
  if m.get('reuse'):continue
  path=root/m['file'];original.setdefault(path,path.read_bytes());s=path.read_text()
  start=s.index(m['scope']) if m.get('scope') else 0
  end=s.find('\n    fn ',start+1) if m.get('scope') else -1
  # Public function boundaries also delimit scopes.
  alt=s.find('\n    pub fn ',start+1) if m.get('scope') else -1
  candidates=[n for n in (end,alt) if n>=0];end=min(candidates) if candidates else len(s)
  region=s[start:end];assert region.count(m['old'])==1,(m['id'],region.count(m['old']))
  path.write_text(s[:start]+region.replace(m['old'],m['new'])+s[end:])
 for m in mutations:
  base=['cargo','nextest','run','--locked']
  if m.get('package'):
   base += ['-p', m['package']]
  else:
   base += ['--workspace','--exclude','html5ever']
  base += ['-j','4','--no-fail-fast']
  env=os.environ.copy();env['CAMPFIRE_LEDGER_MUTATION']=m['id']
  log=a.output/(m['id']+'.log')
  with log.open('w') as f:
   r=subprocess.run(command+base+['-E','test('+m['test']+')'],env=env,stdout=f,stderr=subprocess.STDOUT)
  s=log.read_text();summary=re.findall(r'[^\n]*Summary[^\n]*',s)
  panic,valid=assertion_failure(s,m.get('expected_assertions'))
  killed=r.returncode!=0 and any('1 test run: 0 passed, 1 failed' in l or '1 tests run: 0 passed, 1 failed' in l for l in summary) and valid
  results.append(dict(id=m['id'],test=m['test'],package=m.get('package'),command=command+base+['-E','test('+m['test']+')'],exit_code=r.returncode,killed=killed,summary=summary,panic=panic,log=log.name,log_sha256=hashlib.sha256(log.read_bytes()).hexdigest(),expected_assertions=m.get('expected_assertions')))
  print(m['id'], 'KILLED' if killed else 'NOT ACCEPTED',summary,panic,flush=True)
finally:
 for path,data in original.items():path.write_bytes(data)
 receipt=dict(production_restored=all(p.read_bytes()==b for p,b in original.items()),source_hashes={str(p.relative_to(root)):hashlib.sha256(b).hexdigest() for p,b in original.items()},results=results)
 (a.output/'results.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(f'Mutation summary: {len(results)} run; {sum(r["killed"] for r in results)} killed; {sum(not r["killed"] for r in results)} not accepted; production restored={receipt["production_restored"]}')
sys.exit(0 if len(results)==len(mutations) and all(r['killed'] for r in results) else 1)
