# Pinned controller coercions, including the independent credential save boundary.
require 'action_dispatch/testing/integration'
labels = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'parity/.seed/default/labels.json')))
ApplicationController.allow_forgery_protection = false
ActiveRecord::Base.logger = nil
Rails.logger = ActiveSupport::Logger.new($stderr)
Rails.application.env_config['action_dispatch.show_exceptions'] = :all
Rails.application.env_config['action_dispatch.show_detailed_exceptions'] = false
owner = User.find(labels.fetch('users.david'))
bot = User.find(labels.fetch('users.bender'))
connection = ActiveRecord::Base.connection
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
headers = {'Cookie'=>"session_token=#{labels.fetch('session_cookies.david')}", 'HTTP_USER_AGENT'=>'Mozilla/5.0 Chrome/140.0.0.0', 'Accept'=>'text/html'}
browser.post('/sudo', params:{password:'secret123456'}, headers:)
headers.delete('Cookie')
result = {reference: 'd7c7de92', expiry: [], github_tokens: [], errors: {}}

inputs = ['-292278/02/01','9223372036854775808/02/01','-9223372036854775809/02/01',"R#{'9'*80}.1.2", "'#{'9'*110}", '2147483648:01:02', '01:2147483648:02', '01:02:2147483648', '2147483648pm','2147483648th', '2147483647:01:02','2030-2147483648-01','2030-01-2147483648', '2030-06-15 01:02 +999999999999999999999999']
['UTC','America/New_York'].each do |zone|
 owner.update_columns(time_zone:zone)
 inputs.each_with_index do |input,index|
  [true,false].each do |valid|
   name = valid ? "Extreme #{zone} #{index}" : ''
   before=AgentCredential.count
   audits=AuditLog.where(action:'agent.credential.create').count
   browser.post("/account/bots/#{bot.id}/credentials",params:JSON.generate(agent_credential:{name:, expires_at:input}),headers:headers.merge('Content-Type'=>'application/json'))
   row=connection.select_one("SELECT expires_at FROM agent_credentials WHERE name=#{connection.quote(name)}")
   result[:expiry] << {zone:,attributes:{name:,expires_at:input},status:browser.response.status,persisted:AgentCredential.count>before,stored:row && row['expires_at'],audits:AuditLog.where(action:'agent.credential.create').count-audits}
   if browser.response.status==500
    result[:errors]['500']={body:browser.response.body,content_type:browser.response.headers['Content-Type']}
   end
   AgentCredential.where(name:name).delete_all
  end
 end
end

module ExtremeTokenProbe
 class << self; attr_accessor :token; end
end
Github::WriteClient.singleton_class.prepend(Module.new do
 define_method(:authenticated_login) do |token|
  ExtremeTokenProbe.token=token
  'fixture-machine'
 end
end)
raws=['1e309','-1e309','18446744073709551616','18446744073709551617','-9223372036854775809','1234567890123456789012345678901234567890']
raws += raws.map{|raw| "[#{raw}]"}
raws += ['{"x":18446744073709551616}', '[{"x":-1e309}]','[[1234567890123456789012345678901234567890]]','[null,1e309,null,-9223372036854775809]', '[-0,-0.0,1.00e309]']
raws.each do |raw|
 ExtremeTokenProbe.token=nil
 browser.post("/account/bots/#{bot.id}/github_connection",params:'{"access_token":'+raw+'}',headers:headers.merge('Content-Type'=>'application/json'))
 result[:github_tokens] << {raw:,token:ExtremeTokenProbe.token,status:browser.response.status}
end
puts JSON.pretty_generate(result)
warn "Rails extreme HTTP boundaries: #{result[:expiry].length} compared; #{result[:expiry].count{|r|r[:status]==500}} exceptions; #{result[:expiry].count{|r|r[:persisted]}} saves"
