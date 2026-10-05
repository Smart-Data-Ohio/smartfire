# Exact four assertion gaps reopened by the #244 audit.
require 'json';require 'cgi'
Rails.application.config.hosts.clear;Rails.logger=ActiveSupport::Logger.new($stderr)
ActionView::Base.logger=Rails.logger
ENV['GOOGLE_CLIENT_ID']='parity-client';ENV['GOOGLE_CLIENT_SECRET']='parity-secret'
rows=%w[meeting_error connected_email email_change email_case].map do |name|
 row=nil
 Rails.cache.clear
 ActiveRecord::Base.transaction(requires_new:true) do
  user=User.find(127326141)
  if %w[meeting_error connected_email].include?(name)
   GoogleAccount.where(user_id:user.id).delete_all
   GoogleAccount.create!(user:user,email:'david@gmail.test')
  end
  needle=nil
  if name=='meeting_error'
   user.update!(meeting_status_enabled:true)
   Calendar::MeetingCache.where(user_id:user.id).delete_all
   Calendar::MeetingCache.create!(user:user,fetched_at:Time.current,fetch_error:Calendar::MeetingRefresh::UNREACHABLE_MESSAGE)
   needle=CGI.escapeHTML(Calendar::MeetingRefresh::UNREACHABLE_MESSAGE)
  end
  b=ActionDispatch::Integration::Session.new(Rails.application);b.host! 'campfire.test'
  r=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
  r.cookie_jar.signed.permanent[:session_token]={value:user.sessions.first.token,httponly:true,same_site: :lax};b.cookies['session_token']=r.cookie_jar[:session_token]
  b.get '/users/me/profile';ActiveSupport::IsolatedExecutionState.clear
  params=nil
  if name.start_with?('email_')
   params=name=='email_change' ? {email_address:'david@smartdata.net',current_password:'secret123456'} : {name:'Dave',email_address:'David@37signals.com'}
   csrf=Nokogiri::HTML(b.response.body).at_css('meta[name="csrf-token"]')['content']
   b.put '/users/me/profile',params:{user:params},headers:{'X-CSRF-Token'=>csrf};ActiveSupport::IsolatedExecutionState.clear
  end
  response={status:b.response.status}
  case name
  when 'meeting_error' then response[:notice]=b.response.body.include?(needle)
  when 'connected_email' then response.merge!(connected_email:b.response.body.include?('Connected as david@gmail.test'),disconnect:b.response.body.include?('Disconnect'))
  when 'email_change','email_case' then response.merge!(location:b.response.location,email:user.reload.email_address,name:user.name,marker:user.email_self_changed_at&.iso8601)
  end
  row={name:name,params:params,notice_html:needle,response:response}
  raise ActiveRecord::Rollback
 end
 row
end
puts JSON.pretty_generate(reference:ENV.fetch('PARITY_REFERENCE_SHA'),cases:rows)
warn 'Rails original profile gap oracle: 4 routed cases; original notice, email and save markers'
