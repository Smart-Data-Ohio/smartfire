require 'json'
require 'digest'
Rails.logger=ActiveSupport::Logger.new($stderr)
reference=File.read(File.join(ENV.fetch('PARITY_WORK'),'parity/reference.sha')).strip
raise 'profile oracle requires the pinned reference image' unless ENV['PARITY_REFERENCE_SHA']==reference
JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/users/profile-page-source-hashes.json'))).each {|path,hash|raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest==hash}
class ProfilePageGoldenController < Users::ProfilesController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
user=User.find(127326141)
Current.reset;Current.user=user
renderer=ProfilePageGoldenController.renderer.new(http_host:'campfire.test',https:false,'HTTP_USER_AGENT'=>'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36','rack.session'=>{},'action_dispatch.content_security_policy'=>Rails.application.config.content_security_policy,'action_dispatch.content_security_policy_nonce_generator'=>->(_){'NONCE'})
direct,shared=user.memberships.with_ordered_room.partition {|m|m.room.direct?}
assigns={user:user,direct_memberships:direct,shared_memberships:shared,two_factor_devices:user.two_factor_remembered_devices.live.recent_first.to_a}
html=renderer.render(template:'users/profiles/show',layout:'application',assigns:assigns)
body=renderer.render(template:'users/profiles/show',layout:false,assigns:assigns)
facts={name:user.name,theme:user.theme,text_size:user.text_size,time_zone:user.time_zone,time_zone_explicit:user.time_zone_explicit,tour_completed:user.tour_completed_at.present?,voice_mode:user.voice_mode,push_to_talk_key:user.push_to_talk_key,github_login:user.github_login,github_login_verified:user.github_login_verified?,inbox:user.inbox_preferences.to_h,presence_setting:user.presence_setting,custom_status_emoji:user.custom_status_emoji,custom_status_text:user.custom_status_text,ooo_note:user.ooo_note,manual_dnd:user.manual_dnd_active?,quiet_hours_enabled:user.quiet_hours_enabled?,quiet_hours_start:user.quiet_hours_start,quiet_hours_end:user.quiet_hours_end,meeting_dnd:user.meeting_dnd_enabled?,ooo_notify:user.ooo_notify_enabled?,keywords:user.keyword_alerts.order(:phrase).pluck(:phrase),allowed_people:user.dnd_allowed_people.ordered.pluck(:id,:name),google_drive:user.google_account&.drive?,google_sign_in:Google::SignIn.configured?,google_calendar:Google::Client.configured?,github_app:Github::App.configured?,github_account:user.github_connected_account.present?,fizzy_account:user.fizzy_connected_account.present?}
setup={users:{user.id=>{name:'<b>Profile & "</b>',bio:'<script>bio()</script>',custom_status_text:'<img src=x onerror="status()">',custom_status_emoji:'<b>&</b>',ooo_note:'<i>OOO & "</i>',github_login:'<b>login & "</b>'},149087659=>{name:'<b>DND person & "</b>',bio:'<i>title & "</i>'}},rooms:{486777696=>{name:'<b>Membership & "</b>'}},account:{name:'<b>Account & "</b>'}}
setup[:users].each {|id,attrs|User.find(id).update_columns(attrs)}
setup[:rooms].each {|id,attrs|Room.find(id).update_columns(attrs)}
Account.first.update_columns(setup[:account])
Current.reset;Current.user=user.reload
markup_direct,markup_shared=user.memberships.with_ordered_room.partition {|m|m.room.direct?}
markup_html=renderer.render(template:'users/profiles/show',layout:'application',assigns:assigns.merge(user:user,direct_memberships:markup_direct,shared_memberships:markup_shared))
markup={setup:setup,html:markup_html}
puts JSON.pretty_generate(reference:reference,user_id:user.id,html:html,body:body,facts:facts,brand_icon_names:Icons.client_icon_names,markup:markup)
warn "Rails full profile oracle: 2 complete application pages (seed and markup) and body; real seed memberships and WS9 security; plain pinned reference #{reference}"
