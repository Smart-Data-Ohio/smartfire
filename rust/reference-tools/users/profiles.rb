require "json"
require "digest"
Rails.application.config.hosts.clear
Rails.logger = ActiveSupport::Logger.new($stderr)
%w[app/controllers/users/profiles_controller.rb app/models/user.rb app/models/user/status_settings.rb].each do |file|
  expected=JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"),"reference-tools/users/profiles-source-hashes.json"))).fetch(file)
  raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest==expected
end
user=User.find(127326141)
session=ActionDispatch::Integration::Session.new(Rails.application)
session.host! "campfire.test"
request=ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST"=>"campfire.test","rack.input"=>StringIO.new))
request.cookie_jar.signed.permanent[:session_token]={value:user.sessions.first.token,httponly:true,same_site: :lax}
session.cookies["session_token"]=request.cookie_jar[:session_token]
session.get "/account/edit"
ActiveSupport::IsolatedExecutionState.clear
token=Nokogiri::HTML(session.response.body).at_css('meta[name="csrf-token"]')["content"]
initial={name:"David",bio:nil,theme:"system",text_size:"default",time_zone:nil,time_zone_explicit:false,voice_mode:nil,push_to_talk_key:nil,inbox_preferences:nil,github_login:nil}
verb = ENV.fetch("PROFILE_RECEIPT_VERB", "patch")
raise "unsupported receipt verb" unless %w[patch put].include?(verb)
cases=[
 ["original_form_profile",{name:"John Doe",bio:"Acrobat"}],
 ["original_form_foreign_name",{name:"John Doe"}],
 ["original_form_theme_zone",{theme:"dark",time_zone:"Pacific Time (US & Canada)"}],
 ["original_form_voice",{voice_mode:"push_to_talk",push_to_talk_key:"CapsLock"}],
 ["original_form_github",{github_login:"  David-GH "}],
 ["original_form_github_clear",{github_login:""},{github_login:"david-gh"}],
 ["appearance",{theme:"dark",time_zone:"America/New_York"}],
 ["text_size",{text_size:"larger"}], ["legacy_zone",{time_zone:"Pacific Time (US & Canada)"}],
 ["not_set",{time_zone:""},{time_zone:"America/New_York",time_zone_explicit:true}], ["nil_zone",{time_zone:nil}], ["zone_array",{time_zone:["America/New_York"]}],
 ["bad_theme",{theme:"neon"}], ["bad_zone",{time_zone:"Narnia"}], ["bad_text",{text_size:"huge"}],
 ["voice",{voice_mode:"push_to_talk",push_to_talk_key:"  CapsLock  "}],
 ["bad_voice",{voice_mode:"unknown"}], ["long_key",{push_to_talk_key:"x"*21}],
 ["inbox",{inbox_preferences:{github_review_requests:"0",agent_work:true,unknown:false}}],
 ["bad_inbox",{inbox_preferences:{agent_work:"maybe"}}],
 ["original_github_normalize",{github_login:"  David-GH  "}],
 ["original_github_unlink",{github_login:" "},{github_login:"david-gh"}],
 ["github",{github_login:"  Fixture-Login  "}], ["github_clear",{github_login:" "}],
 ["foreign_path",{theme:"light",user_id:149087659}],
 ["settings_and_name_rollback",{theme:"neon",name:"must not save"}],
 ["nil_settings",{theme:nil,text_size:nil,voice_mode:nil}],
 ["wrong_shapes",{theme:["dark"],voice_mode:{value:"push_to_talk"}}],
 ["inbox_merge",{inbox_preferences:{agent_work:false}},{inbox_preferences:{"event_reminders"=>"0"}}],
 ["verified_login",{github_login:"different"},{github_login:"verified"},true,nil],
 ["blank_disconnect_reason",{github_login:"different"},{github_login:"verified"},true," "],
 ["disconnected_login",{github_login:"different"},{github_login:"verified"},true,"401"],
 ["duplicate_login",{github_login:"shared-login"},{},false,nil,true],
 ["persisted_invalid_settings",{name:"must not save"},{theme:"neon"}],
 ["inbox_array",{inbox_preferences:[{agent_work:false}]}],
 ["inbox_scalar",{inbox_preferences:"wrong"}],
 ["inbox_all_keys",{inbox_preferences:{github_review_requests:"0",agent_approvals:"0",agent_work:"1",event_reminders:"0",huddle_invitations:"0"}}],
 ["core_profile",{name:"John Doe",bio:"Acrobat"}],
 ["foreign_name",{name:"John Doe",bio:"Acrobat"}]
].map do |name,params,before,connected,reason,duplicate|
 user.update_columns(**initial,**(before || {}),updated_at:1.hour.ago)
 GithubConnectedAccount.where(user_id:user.id).delete_all
 if connected
   GithubConnectedAccount.insert_all!([{user_id:user.id,github_login:"verified",access_token:"ws8br2-fixture",disconnected_reason:reason,created_at:Time.current,updated_at:Time.current}])
 end
 User.find(149087659).update_columns(github_login: duplicate ? "shared-login" : nil)
 path=%w[foreign_path foreign_name original_form_foreign_name].include?(name) ? "/users/149087659/profile" : "/users/me/profile"
 options = {params:{user:params},headers:{"X-CSRF-Token"=>token,"Accept"=>"text/html"}}
 options[:as] = :json unless name.start_with?("original_form_")
 session.public_send(verb,path,**options)
 ActiveSupport::IsolatedExecutionState.clear
 state=user.reload.attributes.slice(*initial.keys.map(&:to_s)).merge("other_name"=>User.find(149087659).name,"updated_at"=>user.updated_at.iso8601(6))
 {name:name,path:path,params:params,before:before || {},connection:!!connected,reason:reason,duplicate:!!duplicate,status:session.response.status,state:state}.tap { |row| row[:encoding] = "form" if name.start_with?("original_form_") }
end
puts JSON.pretty_generate(reference: ENV.fetch('PARITY_REFERENCE_SHA'),profiles:cases)
warn "Rails profile settings oracle: #{cases.size} #{verb.upcase} cases; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
