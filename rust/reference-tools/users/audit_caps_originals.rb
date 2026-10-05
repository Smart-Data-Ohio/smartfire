require 'json'
require 'csv'
require 'nokogiri'
Rails.logger=ActiveSupport::Logger.new($stderr)
Rails.application.config.hosts.clear
admin=User.find(127326141)
cases=[['past',2],['within',5]].map do |name,limit|
 AuditLog.delete_all
 %w[user.ban user.role.change].tap{|a|a << 'user.ban' if limit==2}.each{|action|AuditLog.record!(action:action)}
 browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
 request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
 request.cookie_jar.signed.permanent[:session_token]={value:admin.sessions.first.token,httponly:true,same_site: :lax};browser.cookies['session_token']=request.cookie_jar[:session_token]
 browser.get '/account/audit_log';ActiveSupport::IsolatedExecutionState.clear
 csrf=Nokogiri::HTML(browser.response.body).at_css('meta[name="csrf-token"]')['content']
 browser.post '/sudo',params:{password:'secret123456'},headers:{'X-CSRF-Token'=>csrf}
 ActiveSupport::IsolatedExecutionState.clear
 raise 'sudo response' unless browser.response.status==302
 original=Accounts::AuditLogsController::CSV_EXPORT_LIMIT
 Accounts::AuditLogsController.send(:remove_const,:CSV_EXPORT_LIMIT);Accounts::AuditLogsController.const_set(:CSV_EXPORT_LIMIT,limit)
 begin
  browser.get '/account/audit_log';ActiveSupport::IsolatedExecutionState.clear
  raise 'HTML cap response' unless browser.response.status==200
  notice=browser.response.body.include?("newest #{limit}")
  browser.get '/account/audit_log.csv';ActiveSupport::IsolatedExecutionState.clear
  raise 'CSV cap response' unless browser.response.status==200
  disposition=browser.response.headers['Content-Disposition'];rows=CSV.parse(browser.response.body,headers:true).size
  {name:name,response:{notice:notice,truncated:disposition.include?('truncated'),filename_cap:disposition.include?("truncated-to-#{limit}"),rows:rows,matches_table:rows==AuditLog.count}}
 ensure
  Accounts::AuditLogsController.send(:remove_const,:CSV_EXPORT_LIMIT);Accounts::AuditLogsController.const_set(:CSV_EXPORT_LIMIT,original)
 end
end
puts JSON.pretty_generate(reference:ENV.fetch('PARITY_REFERENCE_SHA'),cases:cases)
warn 'Rails original audit caps: 2 declarations; 4 HTTP responses'
