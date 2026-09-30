#!/usr/bin/env python3
"""Inventory unclaimed full-file Rails acceptance. Source case counts are not execution counts."""
import hashlib
import json
import os
from pathlib import Path
import re
import argparse
parser=argparse.ArgumentParser()
parser.add_argument('--test-log',type=Path,help='raw cargo test output for the named one-to-one Rust Rails cases')
parser.add_argument('--rails-log',type=Path,help='raw per-file output from check_controller_files.py')
args=parser.parse_args()
root=Path(__file__).resolve().parents[3]
reference=Path(os.environ.get('CAMPFIRE_REFERENCE',root))
files = [
    'test/controllers/rooms_controller_test.rb',
    'test/controllers/rooms/opens_controller_test.rb',
    'test/controllers/rooms/closeds_controller_test.rb',
    'test/controllers/rooms/directs_controller_test.rb',
    'test/controllers/rooms/involvements_controller_test.rb',
    'test/controllers/rooms/refreshes_controller_test.rb',
    'test/controllers/rooms/reads_controller_test.rb',
    'test/controllers/rooms/members_controller_test.rb',
    'test/controllers/rooms/categories_controller_test.rb',
    'test/controllers/rooms/favorites_controller_test.rb',
    'test/controllers/rooms/inbound_email_addresses_controller_test.rb',
    'test/controllers/room_categories_controller_test.rb',
    'test/controllers/audit_log/rooms_audit_test.rb',
    'test/controllers/public_pages_controller_test.rb',
    'test/controllers/first_runs_controller_test.rb',
    'test/controllers/welcome_controller_test.rb',
    'test/controllers/switchers_controller_test.rb',
    'test/controllers/users_controller_test.rb',
    'test/controllers/users/sidebars_controller_test.rb',
    'test/controllers/users/tours_controller_test.rb',
    'test/controllers/users/profiles_controller_test.rb',
    'test/controllers/users/avatars_controller_test.rb',
    'test/controllers/users/cards_controller_test.rb',
    'test/controllers/users/bans_controller_test.rb',
    'test/controllers/users/time_zones_controller_test.rb',
    'test/controllers/accounts_controller_test.rb',
    'test/controllers/accounts/users_controller_test.rb',
    'test/controllers/accounts/icons_controller_test.rb',
    'test/controllers/accounts/audit_logs_controller_test.rb',
    'test/controllers/accounts/custom_styles_controller_test.rb',
    'test/controllers/accounts/logos_controller_test.rb',
    'test/controllers/accounts/join_codes_controller_test.rb',
    'test/controllers/workspace_icons_controller_test.rb',
    'test/controllers/pwa_controller_test.rb',
    'test/controllers/qr_code_controller_test.rb',
    'test/controllers/unfurl_links_controller_test.rb',
    'test/system/audit_log_test.rb',
    'test/system/channel_members_test.rb',
    'test/system/channel_navigation_test.rb',
    'test/system/first_run_tour_test.rb',
    'test/system/icons_test.rb',
    'test/system/keyboard_shortcuts_test.rb',
    'test/system/member_select_mode_test.rb',
    'test/system/mobile_layout_test.rb',
    'test/system/motion_test.rb',
    'test/system/people_group_dms_test.rb',
    'test/system/quick_switcher_test.rb',
    'test/system/room_header_test.rb',
    'test/system/service_worker_test.rb',
    'test/system/sidebar_organize_test.rb',
    'test/system/sidebar_room_menu_test.rb',
    'test/system/starred_people_test.rb',
    'test/system/timezone_detection_test.rb',
    'test/system/unread_divider_test.rb',
    'test/system/unread_rooms_test.rb',
    'test/system/workspace_icons_test.rb',
    'test/system/browser_launch_profile_test.rb',
    'test/system/content_security_policy_test.rb',
]
handoff_controllers={'public_pages','first_runs','welcome','users','accounts','workspace_icons','pwa','qr_code'}
handoff_systems={'audit_log','first_run_tour','icons','service_worker','timezone_detection','workspace_icons'}
def owner(file):
    if file=='test/controllers/audit_log/rooms_audit_test.rb': return 'WS8br room cases; WS8br2 account cases'
    if file.startswith('test/controllers/users/') and '/sidebars_' not in file: return 'WS8br2'
    if file.startswith('test/controllers/accounts/'): return 'WS8br2'
    base=Path(file).stem.removesuffix('_controller_test').removesuffix('_test')
    if file.startswith('test/controllers/') and base in handoff_controllers: return 'WS8br2'
    if file.startswith('test/system/') and base in handoff_systems: return 'WS8br2'
    if base=='starred_people': return 'WS12 with WS8br2/WS8br presentation'
    return 'WS8br'
result=[]
for file in files:
    path=reference/file
    if not path.exists():
        result.append(dict(file=file,owner=owner(file),reference_file_present=False,declared_cases=[],declared_count=0,rails_tests_run=0,rails_pass_count=0,status='full-file acceptance deferred'))
        continue
    source=path.read_text()
    cases=[]
    for match in re.finditer(r"^\s*(?:test\s+([\"'])(.*?)\1\s+do|def\s+(test_\w+))",source,re.M):
        cases.append(dict(name=match[2] or match[3],line=source.count('\n',0,match.start())+1))
    result.append(dict(file=file,owner=owner(file),reference_file_present=True,source_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),declared_cases=cases,declared_count=len(cases),rails_tests_run=0,rails_pass_count=0,status='full-file acceptance deferred'))
output=dict(reference='d7c7de92',note='These are source declarations, not dynamically expanded Rails tests. Equivalent Rust subsets are reported separately; no full-file Rails acceptance is claimed.',files=result)
if args.test_log:
    receipts=args.test_log.read_text()
    file=next(row for row in result if row['file']=='test/controllers/rooms/inbound_email_addresses_controller_test.rb')
    assert file['source_sha256']=='7f0a2071a6a696f04b5fa27a677e9d929de25abce4aec7653db43c1250ba423e', 'inbound Rails case drift'
    passed=set(re.findall(r'^test controllers::rooms::inbound_rails_cases::(\w+) \.\.\. ok$',receipts,re.M))
    for case in file['declared_cases']:
        selector=re.sub(r'[^a-z0-9]+','_',case['name'].lower()).strip('_')
        assert selector in passed, f'missing Rust pass receipt: {selector}'
        case.update(rust_test=f'controllers::rooms::inbound_rails_cases::{selector}',rust_result='passed')
    file.update(rust_cases_run=len(file['declared_cases']),rust_pass_count=len(file['declared_cases']),status='all eight source-declared cases ported to individually executed Rust tests')
    print(f"Rails case port receipts: {file['file']}: {file['rust_pass_count']} Rust cases passed, 0 deferred; 0 Rails Minitest executions")
    output['note']='Source declarations are not dynamically expanded Rails tests. The inbound-email file has one individually executed Rust test per declaration, with receipts from the supplied raw cargo log; remaining full-file case mappings are deferred. No Rails Minitest execution claimed.'
if args.rails_log:
    text=args.rails_log.read_text()
    receipts=re.findall(r'^(test/controllers/[^\n]+)\n([0-9]+) runs, ([0-9]+) assertions, 0 failures, 0 errors, 0 skips$',text,re.M)
    assert len(receipts)==14,'missing per-file Rails receipts'
    by_file={entry['file']:entry for entry in result}
    for path,runs,assertions in receipts:
        entry=by_file[path]
        entry.update(rails_tests_run=int(runs),rails_pass_count=int(runs),rails_assertions=int(assertions))
    output['note']='Source declarations are distinct from actual executions. Rails reference passes are recorded per file from the supplied raw log; they do not imply Rust case completion. Named Rust ports require their own cargo receipts.'
    print(f'Rails controller reference receipts: {len(receipts)} files, {sum(int(row[1]) for row in receipts)} passes, 0 failures, 0 errors, 0 skips; reference only')
(root/'rust/plans/ws8br-rails-cases.json').write_text(json.dumps(output,indent=2)+'\n')
print(f'Rails deferred inventory: {len(result)} files, {sum(r["declared_count"] for r in result)} source-declared cases; {sum(r["rails_tests_run"] for r in result)} Rails tests run, {sum(r["rails_pass_count"] for r in result)} Rails reference passes; Rust mappings separate')
