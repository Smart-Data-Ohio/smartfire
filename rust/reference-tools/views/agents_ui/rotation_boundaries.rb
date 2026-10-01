# Real pinned HTTP: these writes precede the independent audit insert.
require 'action_dispatch/testing/integration'
labels = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'parity/.seed/default/labels.json')))
ApplicationController.allow_forgery_protection = false
bot = User.find(labels.fetch('users.bender'))
legacy = User.create_bot!(name: 'Boundary legacy', webhook_url: 'https://example.test/receiver')
legacy.webhook.ensure_signing_secret!
bot.agent.ensure_webhook_signing_secret!
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
headers = { 'Cookie' => "session_token=#{labels.fetch('session_cookies.david')}", 'HTTP_USER_AGENT' => 'Mozilla/5.0 Chrome/140.0.0.0' }
browser.post('/sudo', params: { password: 'secret123456' }, headers:)
ActiveRecord::Base.connection.execute("CREATE TRIGGER reject_rotation_audit BEFORE INSERT ON audit_logs WHEN NEW.action IN ('agent.credential.reset','agent.webhook_secret.reset') BEGIN SELECT RAISE(ABORT,'fixture rotation audit rejection'); END;")
[
  [bot, 'key', :put, -> { bot.reload.bot_token_digest }],
  [bot, 'webhook_secret', :post, -> { bot.agent.reload.webhook_signing_secret }],
  [legacy, 'webhook_secret', :post, -> { legacy.webhook.reload.signing_secret }]
].each do |user, action, method, read|
  before = read.call
  begin
    browser.public_send(method, "/account/bots/#{user.id}/#{action}", params: {}, headers: headers.except('Cookie'))
    raise "expected 500, got #{browser.response.status}" unless browser.response.status == 500
  rescue ActiveRecord::StatementInvalid => error
    raise unless error.message.include?('fixture rotation audit rejection')
  end
  after = read.call
  raise "rotation rolled back: #{action}" if after.blank? || before == after
end
puts 'Rails rotation fault boundaries: bot key, agent signing secret, legacy signing secret remain rotated after rejected audit; 3 requests passed'
