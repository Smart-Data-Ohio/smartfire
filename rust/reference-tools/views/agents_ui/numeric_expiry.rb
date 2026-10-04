# Numeric datetime values pass unchanged through Rails' raw SQLite writer.
require 'action_dispatch/testing/integration'
require 'json'
ApplicationController.allow_forgery_protection = false
Rails.application.env_config['action_dispatch.show_exceptions'] = :all
Rails.application.env_config['action_dispatch.show_detailed_exceptions'] = false
ActiveRecord::Base.logger = nil
Rails.logger = ActiveSupport::Logger.new($stderr)
Rails.application.env_config['action_dispatch.logger'] = Rails.logger
ActionController::Base.logger = Rails.logger
ActionView::Base.logger = Rails.logger
labels = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'parity/.seed/default/labels.json')))
owner = User.find(labels.fetch('users.david'))
bot = User.find(labels.fetch('users.bender'))
connection = ActiveRecord::Base.connection
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
headers = { 'Cookie' => "session_token=#{labels.fetch('session_cookies.david')}", 'Accept' => 'text/html', 'Content-Type' => 'application/json', 'HTTP_USER_AGENT' => 'Mozilla/5.0 Chrome/140.0.0.0' }
browser.post('/sudo', params: { password: 'secret123456' }, headers: headers.reject { |k,_| k == 'Content-Type' })
headers.delete('Cookie')
cases = []
['UTC', 'America/New_York'].each do |zone|
 owner.update_columns(time_zone: zone)
 [12, 12.5, -12, 0, true].each_with_index do |input, index|
  name = "Raw expiry #{zone} #{index}"
  before = AuditLog.where(action: 'agent.credential.create').count
  browser.post("/account/bots/#{bot.id}/credentials", params: JSON.generate(agent_credential: { name:, expires_at: input }), headers:)
  row = connection.select_one("SELECT id,expires_at,typeof(expires_at) AS storage_type FROM agent_credentials WHERE name=#{connection.quote(name)}")
  credential = AgentCredential.find(row.fetch('id')) if row
  created_status = browser.response.status
  browser.get("/account/bots/#{bot.id}/credentials", headers: headers.reject { |k,_| k == 'Content-Type' })
  cases << { list_status: browser.response.status, list_body: browser.response.status >= 400 ? browser.response.body : nil, zone:, input:, status: created_status, stored: row && row['expires_at'], storage_type: row && row['storage_type'], read_back: credential&.expires_at, audits: AuditLog.where(action: 'agent.credential.create').count-before }
 end
end
puts JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA"), cases:)
warn "Rails raw credential expiry: #{cases.size} HTTP saves and model reads"
