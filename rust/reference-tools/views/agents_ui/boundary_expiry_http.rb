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
result = {reference: ENV.fetch("PARITY_REFERENCE_SHA"), expiry: [], github_tokens: [], errors: {}}

inputs = ['9999-12-31 00:00:00','-9999-01-01 00:00:00','9999-12-30 23:00:00','10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001-12-31 24:00:00']
['UTC','Asia/Tokyo'].each do |zone|
 owner.update_columns(time_zone:zone)
 inputs.each_with_index do |input,index|
  [true].each do |valid|
   name = valid ? "Extreme #{zone} #{index}" : ''
   before=AgentCredential.count
   audits=AuditLog.where(action:'agent.credential.create').count
   browser.post("/account/bots/#{bot.id}/credentials",params:JSON.generate(agent_credential:{name:, expires_at:input}),headers:headers.merge('Content-Type'=>'application/json'))
   row=connection.select_one("SELECT expires_at FROM agent_credentials WHERE name=#{connection.quote(name)}")
   read_back = begin
    credential=AgentCredential.find_by(name:name)
    value=credential && credential.expires_at
    {stored:value && connection.send(:quoted_date,value)}
   rescue StandardError => error
    {error:error.class.name}
   end
   result[:expiry] << {zone:,attributes:{name:,expires_at:input},status:browser.response.status,persisted:AgentCredential.count>before,stored:row && row['expires_at'],audits:AuditLog.where(action:'agent.credential.create').count-audits,read_back:}
   if browser.response.status==500
    result[:errors]['500']={body:browser.response.body,content_type:browser.response.headers['Content-Type']}
   end
   AgentCredential.where(name:name).delete_all
  end
 end
end

puts JSON.pretty_generate(result)
warn "Rails boundary expiry HTTP: #{result[:expiry].length} cases"
