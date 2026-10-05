# Original AuditLogsControllerTest observations through Rails' registered endpoints.
require 'json'
require 'csv'
Rails.application.config.hosts.clear
Rails.logger=ActiveSupport::Logger.new($stderr)
ActionView::Base.logger=Rails.logger
admin=User.find(127326141);member=User.find(712064548)
def browser_for(user)
 browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
 if user
  request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
  request.cookie_jar.signed.permanent[:session_token]={value:user.sessions.first.token,httponly:true,same_site: :lax}
  browser.cookies['session_token']=request.cookie_jar[:session_token]
 end
 browser
end
def observe(browser,csv=false)
 if csv == 'formula'
  {status:browser.response.status,quoted_formula_target:CSV.parse(browser.response.body,headers:true).any?{|row|row['target'].to_s.start_with?("'=")}}
 elsif csv
  {status:browser.response.status,media_type:browser.response.media_type,body:browser.response.body}
 elsif browser.response.status != 200
  {status:browser.response.status,location:browser.response.location}
 else
  dom=Nokogiri::HTML(browser.response.body)
  {status:browser.response.status,location:browser.response.location,h1:dom.css('h1').map(&:text),codes:dom.css('tbody td code').map(&:text),cells:dom.css('tbody td').map(&:text),rows:dom.css('tbody tr').size,exports:dom.css('a').select{|n|n.text=='Export CSV'}.map{|n|[n['href'],n.text]},older:browser.response.body.include?('Older entries'),newest:browser.response.body.include?('Newest entries')}
 end
end
cases=%w[browse member visitor visitor_csv member_csv actor action unknown dates paging csv formula request_formula].map do |name|
 AuditLog.delete_all
 member.update_columns(name:'Kevin')
 ban=AuditLog.record!(action:'user.ban',actor:admin,target:member,changes:{status:%w[active banned]},ip_address:'203.0.113.7')
 AuditLog.record!(action:'user.role.change',actor:admin,target:member,changes:{role:%w[member administrator]})
 who=name.start_with?('visitor') ? nil : name.start_with?('member') ? member : admin
 browser=browser_for(who)
 if %w[csv formula request_formula].include?(name)
  browser.get '/account/audit_log';ActiveSupport::IsolatedExecutionState.clear
  csrf=Nokogiri::HTML(browser.response.body).at_css('meta[name="csrf-token"]')['content']
  browser.post '/sudo',params:{password:'secret123456'},headers:{'X-CSRF-Token'=>csrf}
  ActiveSupport::IsolatedExecutionState.clear
 end
 paths=case name
 when 'actor'
  AuditLog.record!(action:'user.ban',actor:User.find(149087659),target:member)
  ['/account/audit_log?actor=jason%4037signals.com']
 when 'action'
  AuditLog.record!(action:'room.create',actor:admin,target:Room.find(486777696))
  ['/account/audit_log?audit_action=room.create&target_type=Room','/account/audit_log?audit_action=room.create&target_type=User']
 when 'unknown' then ['/account/audit_log?audit_action=room.nuke&target_type=Spaceship']
 when 'dates'
  AuditLog.where(id:ban.id).update_all(created_at:10.days.ago)
  ["/account/audit_log?from=#{5.days.ago.to_date}","/account/audit_log?to=#{5.days.ago.to_date}"]
 when 'paging'
  60.times{|i|AuditLog.record!(action:'user.ban',actor:admin,target:member,changes:{n:i},ip_address:"10.0.0.#{i%250+1}")}
  ['/account/audit_log','/account/audit_log?page=2']
 when 'csv' then ['/account/audit_log.csv?audit_action=user.ban']
 when 'formula'
  member.update!(name:"=cmd|'/c calc'!A0")
  AuditLog.record!(action:'user.ban',actor:admin,target:member)
  ['/account/audit_log.csv']
 when 'request_formula'
  AuditLog.record!(action:'session.sign_in.failure',actor_label:'mallory@evil.example',ip_address:'198.51.100.9',user_agent:"=cmd|'/c calc'!A0")
  ['/account/audit_log.csv?audit_action=session.sign_in.failure']
 else ["/account/audit_log#{name.end_with?('_csv') ? '.csv' : ''}"]
 end
 responses=paths.map do |path|
  browser.get path;ActiveSupport::IsolatedExecutionState.clear
  {path:path,response:observe(browser,name=='formula' ? 'formula' : %w[csv request_formula].include?(name))}
 end
 {name:name,responses:responses}
end
puts JSON.pretty_generate(reference:ENV.fetch('PARITY_REFERENCE_SHA'),cases:cases)
warn "Rails original audit oracle: #{cases.size} named declarations; #{cases.sum{|c|c[:responses].size}} HTTP responses"
