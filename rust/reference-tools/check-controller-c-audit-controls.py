#!/usr/bin/env python3
"""Actual room/audit/media/gate producer mutations; restore every byte in finally."""
import os,re,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[1]
out=root/'target/controller-c-audit-controls';out.mkdir(parents=True,exist_ok=True)
room='controllers::rooms::original_audit_tests::'
avatar='controllers::users::avatars::original_tests::'
unfurl='controllers::unfurl_links::rails_tests::'
controls=[
 ('room-details','crates/campfire/src/controllers/rooms.rs','changes: Some(changes),','changes: Some(serde_json::json!({"mutant":true})),',[(room+n,case+': original audit') for n,case in [('original_open_creation_actor_target_label_and_details','create'),('original_room_destroy_label_and_actor','destroy'),('original_membership_granted_and_revoked_names','membership'),('original_group_add_jz_has_actor_and_granted_name','add'),('original_last_leaver_kevin_destroys_weekend_plans','last_leave')]]),
 ('invalid-icon','crates/campfire/src/controllers/rooms/closeds.rs','let icon = room_icon_param(c)?.flatten();','let icon: Option<String> = None;',[(room+'original_refused_closed_creation_has_no_audit','failed: original audit')]),
 ('reused-direct','crates/campfire/src/controllers/rooms/directs.rs','if created {','if true {',[(room+'original_group_creation_then_reopening_has_one_audit','direct_reuse: original audit')]),
 ('membership-noop','crates/campfire/src/controllers/rooms/closeds.rs','if granted.is_empty() && revoked.is_empty() {','if false && granted.is_empty() && revoked.is_empty() {',[(room+'original_unchanged_membership_has_no_audit','unchanged: original audit')]),
 ('add-noop','crates/campfire/src/controllers/rooms/directs.rs','Ok(names) if names.is_empty() =>','Ok(names) if false && names.is_empty() =>',[(room+'original_group_add_existing_jason_has_no_audit','add_none: original audit')]),
 ('leave-audit','crates/campfire/src/controllers/rooms.rs','"room.membership.change",','"room.destroy",',[(room+n,case+': original audit') for n,case in [('original_other_leaver_has_no_destroy_audit','leave_no_destroy'),('original_jz_channel_leave_actor_label_and_revocation','leave_channel'),('original_group_leave_david_actor_and_revocation','leave_group')]]),
 ('settings-name','crates/campfire/src/account_security.rs','if before.name != after.name {','if false && before.name != after.name {',[(room+'original_combined_account_settings_audit_fields','settings: original audit')]),
 ('styles-digest','crates/campfire/src/account_security.rs','&digest[..12]','&digest[..11]',[(room+'original_styles_audit_sizes_and_digests','styles: original audit')]),
 ('styles-noop','crates/campfire/src/account_security.rs','if before.custom_styles != account.custom_styles {','if true {',[(room+'original_same_styles_have_no_audit','styles_same: original audit')]),
 ('logo-audit','crates/campfire/src/account_security.rs','json!({"logo":{"before":true,"after":false}})','json!({"logo":{"before":false,"after":true}})',[('controllers::accounts::logos::tests::logo_upload_replacement_deletion_audits_and_cache_validation_match_rails','original logo removal audit pair')]),
 ('logo-size','crates/campfire/src/controllers/accounts/logos.rs','let small = c.param_str("size") == Some("small");','let small = true;',[('controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers','assertion')]),
 ('avatar-text','crates/views/templates/users/avatars/show.svg','{{ initials }}','mutant',[(avatar+'original_kevin_initials_text','original complete Kevin initials SVG')]),
 ('avatar-invalid','crates/campfire/src/controllers/users/avatars.rs','return halt(c.head(campfire_kit::StatusCode::NOT_FOUND));','return halt(c.head(campfire_kit::StatusCode::BAD_REQUEST));',[(avatar+'original_invalid_avatar_token_is_not_found','original invalid avatar status')]),
 ('unfurl-status','crates/campfire/src/controllers/unfurl_links.rs','None => Ok(c.head(StatusCode::NO_CONTENT)),','None => Ok(c.head(StatusCode::OK)),',[(unfurl+n,'assertion') for n in ['original_github_pr_has_no_unfurl','original_fizzy_card_has_no_unfurl','ws15e_rails_composer_missing_tags','ws15e_rails_composer_only_markup']]),
 ('unfurl-json','crates/campfire/src/controllers/unfurl_links.rs','Unfurl::Json(json) => Ok(Some(json)),','Unfurl::Json(json) => Ok(Some(json.replace("Hey!","mutant"))),',[(unfurl+n,'assertion') for n in ['ws15e_rails_composer_create','ws15e_rails_composer_encoded_markup','ws15e_rails_composer_twitter','ws15e_rails_composer_x']]),
 ('unfurl-error','crates/campfire/src/controllers/unfurl_links.rs','Ok(c.params.require("url")?.as_str().map(str::to_string))','Err(Error::BadRequest("mutant".into()))',[(unfurl+'ws15e_rails_composer_missing_url','original ParameterMissing(url)')]),
 ('stale-session','crates/campfire/src/concerns.rs','if enabled {\n        terminate_current_session(c).await?;','if enabled {\n        /* producer defect: leave restored session alive */',[('controllers::accounts::icons::original_tests::original_password_session_enrollment_and_stale_icon_access','assertion')]),
]
originals={}
try:
 for name,relative,before,after,tests in controls:
  p=root/relative;s=p.read_text();count=s.count(before)
  assert count==1,(name,count);s=s.replace(before,after)
  originals.setdefault(p,p.read_bytes());p.write_text(s)
 tests=[(name,t,marker) for name,_,_,_,rows in controls for t,marker in rows]
 env=os.environ.copy();env.update(CI='1',RUST_TEST_THREADS='4',CAMPFIRE_REFERENCE=str(root.parent))
 expr=' or '.join('test(='+t+')' for _,t,_ in tests)
 r=subprocess.run(['cargo','nextest','run','--locked','-p','campfire','-j','4','--no-fail-fast','-E',expr],cwd=root,env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
 (out/'mutants.log').write_text(r.stdout)
 assert r.returncode==100 and 'Summary' in r.stdout,r.stdout[-8000:]
 for name,test,marker in tests:
  assert re.search(r'^\s*FAIL\s+\[[^\]]+\].*'+re.escape(test)+r'$',r.stdout,re.M),(name,'producer survived')
  block=r.stdout.split('test '+test+' ... FAILED',1)[1].split('\n        FAIL',1)[0]
  assert marker in block and 'assertion' in block,(name,'invalid control',block[-3000:])
  assert 'invalid transport' not in block,(name,'transport failure')
  print('C audit control '+name+': '+test+' rejected the intended defect')
 print(f'C audit/media discrimination: {len(controls)} producer mutations; {len(tests)} distinct tests rejected; 0 invalid controls')
finally:
 for path,source in originals.items():path.write_bytes(source)
