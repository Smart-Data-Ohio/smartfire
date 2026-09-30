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
    if base=='unfurl_links': return 'WS15e; WS8br upstream controller re-diff only'
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
    groups=[
        ('test/controllers/rooms/inbound_email_addresses_controller_test.rb','inbound_rails_cases',{},{}),
        ('test/controllers/rooms/directs_controller_test.rb','directs_rails_cases',
         {'create':'create_case',
          "destroy can't reach a closed room the member didn't create":'destroy_cant_reach_a_closed_room_the_member_didnt_create',
          "destroy can't reach an open room the member didn't create":'destroy_cant_reach_an_open_room_the_member_didnt_create',
          "destroy can't reach a room the member isn't in at all":'destroy_cant_reach_a_room_the_member_isnt_in_at_all'},
         {'a member can rename the group and everyone sees the compact system note':'WS8bm message-list rendering; HTTP rename/domain covered separately',
          'group DM notes cannot be edited or deleted':'WS8bm message edit/delete authorization'}),
        ('test/controllers/rooms_controller_test.rb','rooms_rails_cases',
         {"index redirects to the user's last room":'index_redirects_to_the_users_last_room','show':'show_case'},
         {'show renders collapsed work-thread guidance in the new-thread panel':'WS8bm thread-panel rendering',
          'show renders a link preview written by hand without its off-scheme image and link':'WS8bm message renderer and WS15e embed provider',
          'show renders a link preview written by hand without its image pointed at this Smartfire':'WS8bm message renderer and WS15e embed provider',
          'show renders an unfurled link preview':'WS8bm message renderer and WS15e embed provider',
          'show renders the unread divider above the first unread message on the page':'WS8bm list seam; WS8br divider facts are covered separately',
          'show keeps the last page when the first unread fell off it and links the pill to it':'WS8bm full list integration; WS8br page/divider facts covered separately',
          'destroy succeeds when the queue is down and the sweep recovers the room':'Lead decision 2 requires atomic queue rollback; native fault-injection coverage is separate'}),
        ('test/controllers/rooms/opens_controller_test.rb','opens_rails_cases',
         {'new':'new_case','create':'create_case','update':'update_case',
          "a direct room can't be promoted to open by its creator":'a_direct_room_cant_be_promoted_to_open_by_its_creator',
          "a direct room can't be promoted to open by an administrator either":'a_direct_room_cant_be_promoted_to_open_by_an_administrator_either'},{}),
    ]
    for path,module,renamed,deferred in groups:
        file=next(row for row in result if row['file']==path)
        passed=set(re.findall(r'^test controllers::rooms::'+module+r'::(\w+) \.\.\. ok$',receipts,re.M))
        for case in file['declared_cases']:
            if case['name'] in deferred:
                case.update(rust_result='deferred',deferred_to=deferred[case['name']])
                continue
            selector=renamed.get(case['name'],re.sub(r'[^a-z0-9]+','_',case['name'].lower()).strip('_'))
            assert selector in passed,f'missing Rust pass receipt: {module}::{selector}'
            case.update(rust_test=f'controllers::rooms::{module}::{selector}',rust_result='passed')
        expected=len(file['declared_cases'])-len(deferred)
        assert len(passed)==expected, f'unmapped Rust case receipt: {module}'
        file.update(rust_cases_run=expected,rust_pass_count=expected,rust_cases_deferred=len(deferred),status=f'{expected} source-declared cases individually executed in Rust; {len(deferred)} deferred')
        print(f"Rails case port receipts: {path}: {expected} Rust cases passed, {len(deferred)} deferred; Rails reference executions recorded separately")
    output['note']='Source declarations and named Rust ports are distinct from Rails Minitest executions. Supplied raw cargo receipts prove only the individually mapped Rust cases; remaining cases have explicit owners.'
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
