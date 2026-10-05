#!/usr/bin/env python3
"""Original profile/secret/header assertions reject actual producer defects."""
import os,re,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[1];out=root/'target/controller-c-profile-controls';out.mkdir(parents=True,exist_ok=True)
prefix='controllers::users::profile_gap_original_tests::'
controls=[
('meeting-notice','crates/campfire/src/controllers/users/profiles.rs','sections.status.fetch_error = settings_form.fetch_error.clone().filter(|s| !campfire_richtext::ruby::is_blank(s));','sections.status.fetch_error = None;',prefix+'original_meeting_unreachable_notice','meeting_error: exact original profile gap'),
('connected-email','crates/campfire/src/controllers/presenters/profile_sections.rs','fields.google.email = account.email;','fields.google.email = "mutant".into();',prefix+'original_connected_david_gmail_email','connected_email: exact original profile gap'),
('email-marker','crates/campfire/src/authentication.rs','changes.email_self_changed_at = Some(tx.now());','changes.email_self_changed_at = None;',prefix+'original_email_change_with_current_password_marks_now','email_change: exact original profile gap'),
('profile-name','crates/campfire/src/controllers/users/profiles.rs','name: present("name"),','name: None,',prefix+'original_case_only_email_and_dave_name_leave_marker_null','email_case: exact original profile gap'),
('icon-validator','crates/campfire/src/controllers/workspace_icons.rs','&format!("\\"{}\\"", blob.checksum.as_deref().unwrap_or(""))','"mutant"','controllers::accounts::icons::tests::serving_svg_and_png_uses_private_bytes_and_checksum_conditionals','assertion'),
]
originals={}
try:
 for name,relative,before,after,test,marker in controls:
  p=root/relative;s=p.read_text();assert s.count(before)==1,(name,s.count(before));originals.setdefault(p,p.read_bytes());p.write_text(s.replace(before,after))
 env=os.environ.copy();env.update(CI='1',RUST_TEST_THREADS='4',CAMPFIRE_REFERENCE=str(root.parent))
 expr=' or '.join('test(='+t+')' for _,_,_,_,t,_ in controls)
 r=subprocess.run(['cargo','nextest','run','--locked','-p','campfire','-j','4','--no-fail-fast','-E',expr],cwd=root,env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
 (out/'mutants.log').write_text(r.stdout)
 assert r.returncode==100 and 'Summary' in r.stdout,r.stdout[-5000:]
 for name,_,_,_,test,marker in controls:
  assert re.search(r'^\s*FAIL\s+\[[^\]]+\].*'+re.escape(test)+r'$',r.stdout,re.M),(name,'producer survived')
  block=r.stdout.split('test '+test+' ... FAILED',1)[1].split('\n        FAIL',1)[0]
  assert marker in block and 'assertion' in block,(name,'invalid control',block[-1500:])
  print('C profile control '+name+': '+test+' rejected intended defect')
 print(f'C profile discrimination: {len(controls)} producer mutations; {len(controls)} tests rejected; 0 invalid controls')
finally:
 for path,source in originals.items():path.write_bytes(source)
