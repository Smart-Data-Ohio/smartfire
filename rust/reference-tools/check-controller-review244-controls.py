"""Mutation-check PR244 setup and exact routed DOM assertions, restoring each producer in finally."""
import argparse,json,os,re,subprocess
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('phase',choices=['before','after']);p.add_argument('--only');args=p.parse_args()
root=Path(__file__).resolve().parents[2];scratch=Path(os.environ.get('REVIEW244_SCRATCH', root/'rust/target/controller-review244-controls'));scratch.mkdir(parents=True,exist_ok=True)
tests={
 'zone-clear':'controllers::users::profile_settings_tests::manual_profile_settings_match_pinned_rails_patch_vectors',
 'about-comment':'controllers::public_pages::sign_in_google_tests::configured_sign_in_keeps_public_links_and_matches_complete_rails_bodies',
 'unconfigured-about-comment':'controllers::public_pages::tests::unconfigured_sign_in_links_all_public_pages_in_new_tabs',
 'nav-ancestry':'controllers::public_pages::sign_in_google_tests::configured_sign_in_keeps_public_links_and_matches_complete_rails_bodies',
 'directory-presence-comment':'controllers::users::people_tests::index_lists_active_members_with_presence_and_selection',
 'directory-badge-comment':'controllers::users::people_tests::index_lists_active_members_with_presence_and_selection',
 'directory-row-comment-class':'controllers::users::people_tests::index_lists_active_members_with_presence_and_selection',
 'image-aria-label':'controllers::users::people_tests::profile_message_buttons_carry_the_accessible_name',
 'calendar-connect-comment':'controllers::users::profile_sections_tests::configured_calendar_profile_uses_real_account_metadata_and_forms',
 'calendar-reconnect-comment':'controllers::users::profile_sections_tests::live_status_sections_show_cache_errors_disconnects_and_manual_return_date',
 'ban-label-suffix':'controllers::users::people_tests::profile_message_buttons_carry_the_accessible_name',
 'light-meta-comment':'controllers::users::layout_preferences_tests::layout_theme_zone_and_manual_sound_state',
 'own-status-frame':'controllers::users::people_tests::own_card_offers_editing_your_profile',
 'peer-status-extra':'controllers::users::people_tests::card_shows_identity_presence_role_and_actions_for_a_peer',
}
helper=r'''
fn review244_html(html: String) -> String {
    let mutation = std::env::var("WS11UI_REVIEW244_MUTATION").unwrap_or_default();
    let pattern = match mutation.as_str() {
        "about-comment" | "unconfigured-about-comment" => Some(r#"(<a[^>]*href="/about"[^>]*>About</a>)"#),
        "directory-presence-comment" => Some(r#"(<span class="people-directory__presence">[\s\S]*?</span>)"#),
        "directory-badge-comment" => Some(r#"(<span class="profile-card__badge">Agent</span>)"#),
        "calendar-connect-comment" => Some(r#"(<a[^>]*href="#google-calendar-title"[^>]*>Connect Google Calendar</a>)"#),
        "calendar-reconnect-comment" => Some(r#"(<a[^>]*href="#google-calendar-title"[^>]*>Reconnect below</a>)"#),
        "light-meta-comment" => Some(r#"(<meta name="color-scheme" content="light">)"#),
        _ => None,
    };
    if let Some(pattern) = pattern {
        return regex::Regex::new(pattern).unwrap().replace_all(&html, "<!--$1-->").into_owned();
    }
    match mutation.as_str() {
        "nav-ancestry" => html.replace("aria-label=\"About this workspace\"", "aria-label=\"About this workspace\" data-mutant=\"true\"")
            .replace("<nav class=\"txt-align-center txt-small margin-block-start\"", "<section class=\"txt-align-center txt-small margin-block-start\"")
            .replace("</nav>", "</section><!--</nav>-->"),
        "directory-row-comment-class" => html.replace("class=\"people-directory__row", "class=\"mutant-row") + "<!-- class=\"people-directory__row --><!-- class=\"people-directory__row -->",
        "image-aria-label" => html.replace("<img ", "<img aria-label=\"mutant\" "),
        "ban-label-suffix" => html.replace("Ban Kevin", "Ban Kevin extra"),
        _ => html,
    }
}
'''
# Rust raw strings containing href="# need two hashes.
helper=helper.replace('Some(r#"(<a[^>]*href="#google-calendar-title"','Some(r##"(<a[^>]*href="#google-calendar-title"').replace('Calendar</a>)"#)','Calendar</a>)"##)').replace('below</a>)"#)','below</a>)"##)')
originals={}
def edit(path,old,new,count=1):
 path=root/path;source=path.read_text();assert source.count(old)==count,(str(path),old,source.count(old));originals.setdefault(path,path.read_bytes());path.write_text(source.replace(old,new))
results=[]
try:
 if args.phase=='before':
  for relative in ['rust/crates/campfire/src/controllers/public_pages.rs','rust/crates/campfire/src/controllers/public_pages/sign_in_google_tests.rs','rust/crates/campfire/src/controllers/users/people_tests.rs','rust/crates/campfire/src/controllers/users/profile_settings_tests.rs','rust/crates/campfire/src/controllers/users/profile_sections_tests.rs','rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs','rust/vectors/users_profile_settings.json']:
   path=root/relative;originals[path]=path.read_bytes();path.write_bytes(subprocess.check_output(['git','show','f05c3f2a:'+relative],cwd=root))
 edit('rust/crates/db/src/models/user.rs','&& zone != current_zone','&& zone != current_zone\n            && !(std::env::var("WS11UI_REVIEW244_MUTATION").as_deref() == Ok("zone-clear") && zone.is_none())')
 edit('rust/crates/campfire/src/controllers/presenters/view_context.rs','c.render(status, &format::HTML, html)','c.render(status, &format::HTML, review244_html(html))',2)
 path=root/'rust/crates/campfire/src/controllers/presenters/view_context.rs';path.write_text(path.read_text()+helper)
 edit('rust/crates/campfire/src/controllers/presenters/page.rs','let html = layout.render(c, render)?;',r'''let mut html = layout.render(c, render)?;
    match std::env::var("WS11UI_REVIEW244_MUTATION").as_deref() {
        Ok("own-status-frame") => html = html.replace("href=\"/users/me/status/edit\"", "href=\"/users/me/status/edit\" data-turbo-frame=\"_top\""),
        Ok("peer-status-extra") => html = html.replace("</turbo-frame>", "<a href=\"/users/me/status/edit\">Set a status</a></turbo-frame>"),
        _ => (),
    }''')
 command=['cargo','nextest','run','--manifest-path','rust/Cargo.toml','--locked','-p','campfire','-j','4','--no-fail-fast','-E']
 for name,test in tests.items():
  if args.only and not name.startswith(args.only):continue
  env=os.environ.copy();env['WS11UI_REVIEW244_MUTATION']=name
  run=subprocess.run(command+[f'test(={test})'],cwd=root,env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  log=scratch/f'{args.phase}-{name}.log';log.write_text(run.stdout)
  expected = {
   'zone-clear':'not_set', 'about-comment':'exact nav public link selector',
   'unconfigured-about-comment':'exact nav public link selector', 'nav-ancestry':'exact About this workspace nav selector',
   'directory-presence-comment':'original Online presence selector', 'directory-badge-comment':'original Agent badge selector',
   'directory-row-comment-class':'original minimum two directory row elements', 'image-aria-label':'aria-label',
   'calendar-connect-comment':'Connect Google Calendar', 'calendar-reconnect-comment':'original reconnect link selector',
   'ban-label-suffix':'Ban Kevin', 'light-meta-comment':'original light color-scheme meta selector',
   'own-status-frame':'status link uses normal navigation', 'peer-status-extra':'only your own card offers Set a status',
  }[name] if args.phase=='after' else {
   'own-status-frame':'complete routed turbo-frame', 'peer-status-extra':'complete routed turbo-frame', 'image-aria-label':'aria-label',
  }.get(name, 'not_set' if name=='zone-clear' else 'assertion')
  segment = run.stdout.split(f"thread '{test}'", 1)[-1]
  failed=run.returncode==100 and f"thread '{test}'" in run.stdout and expected in segment
  if args.phase=='before' and name.startswith('directory-') and '!html.contains(\"JZ\")' in segment:
   failed=False  # Unrelated random-CSRF substring overlap is not a rejected producer defect.
  passed=run.returncode==0 and re.search(r'^\s*PASS\s+\[[^\]]+\].*'+re.escape(test)+'$',run.stdout,re.M)
  if not (failed or passed): raise AssertionError((name,'invalid mutation or setup',run.returncode,run.stdout[-6000:]))
  results.append({'control':name,'test':test,'result':'rejected' if failed else 'survived','summary':next(s.strip() for s in reversed(run.stdout.splitlines()) if 'Summary [' in s),'log':str(log)})
  print(args.phase,name,results[-1]['result'],results[-1]['summary'],flush=True)
  if args.phase=='after' and not failed:raise AssertionError((name,'surviving mutation'))
finally:
 for path,body in originals.items():path.write_bytes(body)
 (scratch/f'{args.phase}-mutations{"-"+args.only if args.only else ""}.json').write_text(json.dumps(results,indent=2)+'\n')
if args.phase=='after':print(f'PR244 review controls: {len(results)} actual producer defects rejected by {len({r["test"] for r in results})} distinct tests; 0 invalid controls')
