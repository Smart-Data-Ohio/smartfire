#!/usr/bin/env python3
"""Reject actual CSV selection, warning and filename defects in reduced-cap actions."""
import os,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2]
source=root/'crates/campfire/src/controllers/accounts/audit_logs.rs'
baseline=source.read_text()
mutations=[
 ('CSV selection ignores cap','if csv { csv_export_limit }','if csv { browsing::CSV_EXPORT_LIMIT }','original_two_row_cap'),
 ('truncation warning and filename omitted','let truncated = count > csv_export_limit;','let truncated = false;','original_two_row_cap'),
 ('truncation warning and filename falsely present','let truncated = count > csv_export_limit;','let truncated = true;','original_five_row_cap'),
]
try:
 for name,old,new,test in mutations:
  assert baseline.count(old)==1,(name,'source guard')
  source.write_text(baseline.replace(old,new))
  result=subprocess.run(['cargo','nextest','run','--manifest-path',str(root/'Cargo.toml'),'--locked','-j','4','-p','campfire','-E',f'test({test})'],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  # Compiler, fixture, transport and setup failures cannot count as a mutation receipt.
  assert result.returncode==100 and '1 test run: 0 passed, 1 failed' in result.stdout and 'exact original reduced-cap clauses' in result.stdout,(name,result.stdout[-4000:])
  print(f'Original cap control: {name}: intended test rejected',flush=True)
finally:source.write_text(baseline)
print('Original audit cap discrimination: 3 producer mutations; 2 distinct tests rejected; 0 invalid controls',flush=True)
