#!/usr/bin/env python3
"""Mutate the actual producers for every new C native receipt, and restore them."""
import os,re,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[1]
out=root/'target/controller-c-controls';out.mkdir(parents=True,exist_ok=True)
side='controllers::users::sidebar_original_tests::'
audit='controllers::accounts::audit_logs::original_tests::'
account='controllers::accounts::original_control_tests::'
icons='controllers::accounts::icons::original_tests::'
profile='controllers::users::profile_settings_tests::'
controls=[
 ('stack-class','crates/views/src/huddle.rs','class.push_str(" voice-stack--huddle");','class.push_str(" voice-stack--mutant");',[(side+n,'complete routed') for n in ['original_channel_live_stack_names_count_and_attributes','original_board_live_stack_names_count_and_attributes','original_direct_live_stack_peer_count_and_attributes','original_quiet_rows_have_empty_hidden_stack_targets','original_group_direct_name_and_live_stack']]),
 ('participant-cache','crates/campfire/src/controllers/users/sidebars/composition.rs','Some(&participants), user.is_administrator()','Some(&Vec::<i64>::new()), user.is_administrator()',[(side+'original_cached_direct_rename_then_join_invalidates_only_on_participants','complete routed')]),
 ('gateway-gate','crates/campfire/src/huddle.rs','|| self.gateway_secret.is_none()','|| false',[(side+'original_unconfigured_huddle_has_no_stacks_or_presence_controllers','unconfigured stack selector')]),
 ('row-select','crates/campfire/src/controllers/users/sidebars/composition.rs','let row = row(app, conn, user,','let _: i64 = conn.query_row("SELECT id FROM rooms WHERE id=?", [room.id], |r| r.get(0))?;\n        let row = row(app, conn, user,',[(side+'original_mixed_quiet_sidebar_read_counts_stay_flat_with_one_grants_query','quiet-room request SELECT growth')]),
 ('audit-row','crates/views/templates/accounts/audit_logs/show.html','{{ entry.action }}','mutant.action',[(audit+n,case+':') for n,case in [('original_admin_browse_labels_and_export_link','browse'),('original_actor_filter_matches_label_email','actor'),('original_action_and_target_type_filters','action'),('original_unknown_filters_are_ignored','unknown'),('original_date_range_excludes_the_other_action','dates'),('original_paging_links_and_row_count','paging')]]),
 ('csv-header','crates/campfire/src/controllers/accounts/audit_logs.rs','time,action,actor,target_type,target,changes,ip_address,user_agent\\n','time,mutant,actor,target_type,target,changes,ip_address,user_agent\\n',[(audit+'original_filtered_csv_headers_labels_changes_and_ip','csv:')]),
 ('formula','crates/campfire/src/controllers/accounts/audit_logs.rs',"format!(\"'{s}\")",'s.into()',[(audit+'original_formula_target_is_quoted','formula:'),(audit+'original_formula_request_column_is_quoted','request_formula:')]),
 ('authorization','crates/campfire/src/concerns.rs','current_user(c).is_some_and(|user| user.can_administer(None, false))','current_user(c).is_some_and(|_user| true)',[(audit+'original_members_forbidden','member:'),(audit+'original_members_cannot_export','member_csv:'),(icons+'original_jz_cannot_list_create_or_remove_the_existing_icon','assertion')]+[(account+n,case+':') for n,case in [('original_member_cannot_change_admin_role','member_role'),('original_member_cannot_remove_admin','member_remove'),('original_member_styles_refused','member_styles'),('original_jz_cannot_reset_join_code','member_join'),('original_kevin_cannot_ban_jz','member_ban'),('original_kevin_cannot_unban_jz','member_unban')]]),
 ('visitor-destination','crates/campfire/src/concerns.rs','let location = c.url_for(&campfire_routes::new_session());','let location = c.url_for("/account/edit");',[(audit+'original_visitors_redirect_to_sign_in','visitor:'),(audit+'original_visitors_cannot_export','visitor_csv:')]),
 ('icon-label','crates/campfire/src/controllers/accounts/icons.rs','label: Some(format!(":{}:", icon.name))','label: Some(icon.name.clone())',[(icons+'original_acme_upload_list_and_audit_shortcodes','assertion')]),
 ('role','crates/campfire/src/controllers/accounts/users.rs','Some("administrator") => Role::Administrator','Some("administrator") => Role::Member',[(account+'original_admin_self_role_is_preserved','role_self:')]),
 ('deactivation','crates/campfire/src/controllers/accounts/users.rs','crate::authentication::deactivate_user(tx, &mut user, &audit)','{ let _ = (tx, &mut user, &audit); Ok(()) }',[(account+'original_self_removal_active_count_and_lookup','remove_self:')]),
 ('styles-status','crates/campfire/src/controllers/accounts/custom_styles.rs','framed_page!(c, StatusCode::OK','framed_page!(c, StatusCode::CREATED',[(account+'original_styles_edit_success','styles_edit:')]),
 ('styles-writer','crates/campfire/src/controllers/accounts/custom_styles.rs','account.update(tx, None, custom_styles.as_ref().map(|styles| styles.as_deref()), None)?;','let _ = custom_styles; account.update(tx, None, None, None)?;',[(account+'original_styles_exact_value_is_saved','styles:')]),
 ('join-code','crates/campfire/src/controllers/accounts/join_codes.rs','account.reset_join_code(tx)?;','let _ = (tx, &mut account);',[(account+'original_join_code_changes_and_redirects','join:')]),
 ('ban','crates/campfire/src/controllers/users/bans.rs','crate::authentication::set_user_banned(tx, &mut user, true, &audit)','{ let _ = (tx, &mut user, &audit); Ok(()) }',[(account+'original_ban_has_two_exact_ip_records','ban_two:'),(account+'original_ban_destroys_the_single_session','ban_session:')]),
 ('unban','crates/campfire/src/controllers/users/bans.rs','crate::authentication::set_user_banned(tx, &mut user, false, &audit)','{ let _ = (tx, &mut user, &audit); Ok(()) }',[(account+'original_unban_deletes_record_and_activates','unban:')]),
 ('github-login','crates/db/src/models/user/profile_settings.rs','let login = unicode::downcase(strip(&login));','let login = format!("mutant{login}");',[(profile+'original_github_login_normalizes_david_gh','original_github_normalize'),(profile+'original_linked_github_login_is_cleared','original_github_unlink')]),
 ('public-operator','crates/views/templates/public_pages/about.html','{{ operator_name }}','mutant operator', [('controllers::public_pages::tests::public_page_bodies_match_rails_with_operator_escaping_and_email_uri_encoding','first differing byte')]),
]
def verify(log):
 tests=[(name,test,marker) for name,_,_,_,rows in controls for test,marker in rows]
 assert 'Summary' in log and '40 failed' in log,log[-6000:]
 for name,test,marker in tests:
  assert re.search(r'^\s*FAIL\s+\[[^\]]+\].*'+re.escape(test)+r'$',log,re.M),(name,'test did not reject')
  block=log.split('test '+test+' ... FAILED',1)[1].split('\n        FAIL',1)[0]
  assert marker in block and ('assertion' in block or name=='public-operator'),(name,'invalid rejection',block[-1600:])
  print('C producer control '+name+': '+test+' rejected the intended defect')
 print(f'C receipt discrimination: {len(controls)} producer mutations; {len(tests)} distinct tests rejected; 0 invalid controls')
if __name__=='__main__':
 import argparse
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check-log',type=Path);args=parser.parse_args()
 if args.check_log:
  verify(args.check_log.read_text())
 else:
  originals={}
  try:
   for name,relative,before,after,tests in controls:
    p=root/relative;s=p.read_text();expected=2 if name=='icon-label' else 1
    assert s.count(before)==expected,(name,s.count(before));originals.setdefault(p,p.read_bytes());p.write_text(s.replace(before,after))
   tests=[test for _,_,_,_,rows in controls for test,_ in rows]
   env=os.environ.copy();env.update(CI='1',RUST_TEST_THREADS='4',CAMPFIRE_REFERENCE=str(root.parent))
   expr=' or '.join('test(='+t+')' for t in tests)
   r=subprocess.run(['cargo','nextest','run','--locked','-p','campfire','-j','4','--no-fail-fast','-E',expr],cwd=root,env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
   (out/'mutants.log').write_text(r.stdout)
   assert r.returncode==100,r.stdout[-6000:]
   verify(r.stdout)
  finally:
   for path,source in originals.items():path.write_bytes(source)
