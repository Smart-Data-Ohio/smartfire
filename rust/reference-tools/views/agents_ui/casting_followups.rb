# Pinned Date/JSON casts and real credential/sudo requests for PR196's R4 review.
require 'action_dispatch/testing/integration'
input = JSON.parse(File.read(File.join(__dir__, '../../../test-support/agents_ui/casting_followups_inputs.json')))
result = {reference: ENV.fetch("PARITY_REFERENCE_SHA")}
result[:expiry] = input['zones'].flat_map do |zone|
  Time.use_zone(zone) do
    input['expiry'].each_with_index.map do |text, index|
      begin
        value = AgentCredential.new(expires_at: text).expires_at
        {zone:, input_index: index, stored: value && ActiveRecord::Base.connection.send(:quoted_date, value), render: value && value.iso8601}
      rescue StandardError => error
        {zone:, input_index: index, error: error.class.name}
      end
    end
  end
end
labels = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'parity/.seed/default/labels.json')))
ApplicationController.allow_forgery_protection = true
ActiveRecord::Base.logger = nil
Rails.logger = ActiveSupport::Logger.new($stderr)
Rails.application.env_config['action_dispatch.show_exceptions'] = :all
Rails.application.env_config['action_dispatch.show_detailed_exceptions'] = false
# Fix rendering entropy; the requests still run the real session and sudo controllers.
ApplicationController.prepend(Module.new do
  # The vector fixes tokens emitted by forms; verification has separate coverage.
  def verify_authenticity_token = nil

  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end)
Rails.application.env_config['action_dispatch.content_security_policy_nonce_generator'] = ->(_request) { 'NONCE' }
owner = User.find(labels.fetch('users.david'))
bot = User.find(labels.fetch('users.bender'))
headers = {'Cookie'=>"session_token=#{labels.fetch('session_cookies.david')}", 'HTTP_USER_AGENT'=>'Mozilla/5.0 Chrome/140.0.0.0', 'Accept'=>'text/html'}
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
browser.post('/sudo', params: {password: 'secret123456'}, headers:)
headers.delete('Cookie')
connection = ActiveRecord::Base.connection
result[:save_list] = input['render_cases'].each_with_index.map do |spec, index|
  owner.update_columns(time_zone: spec['zone'])
  name = "R4 boundary #{index}"
  browser.post("/account/bots/#{bot.id}/credentials", params: JSON.generate(agent_credential: {name:, expires_at: spec['input']}), headers: headers.merge('Content-Type'=>'application/json'))
  save_status = browser.response.status
  credential = AgentCredential.find_by!(name:)
  stored = connection.select_value("SELECT expires_at FROM agent_credentials WHERE id=#{credential.id}")
  browser.get("/account/bots/#{bot.id}/credentials", headers:)
  expiry_tag = browser.response.body.scan(/· expires (<time\b[^>]*>.*?<\/time>)/m).last&.first
  row = {input: spec['input'], zone: spec['zone'], save_status:, list_status: browser.response.status, stored:, expiry_tag:}
  credential.delete
  row
end
result[:sudo] = input['sudo_numbers'].map do |raw|
  browser = ActionDispatch::Integration::Session.new(Rails.application)
  browser.host! 'campfire.test'
  h = headers.merge('Cookie'=>"session_token=#{labels.fetch('session_cookies.david')}", 'Content-Type'=>'application/json')
  browser.post("/account/bots/#{bot.id}/github_connection?access_token=good", params: '{"unused":'+raw+'}', headers: h)
  challenge_status = browser.response.status
  browser.post('/sudo', params: {password: 'secret123456'}, headers: headers)
  form = browser.response.body[/<form\b[^>]*data-controller="auto-submit"[^>]*>.*?<\/form>/m]
  {raw:, challenge_status:, status: browser.response.status, form:}
end
result[:size] = input['size_digits'].map do |digits|
  raw = '1' * digits
  params = {'unused'=>JSON.parse(raw)}
  bytes = params.to_json.bytesize
  browser = ActionDispatch::Integration::Session.new(Rails.application)
  browser.host! 'campfire.test'
  browser.post("/account/bots/#{bot.id}/github_connection?access_token=good", params: '{"unused":'+raw+'}', headers: headers.merge('Cookie'=>"session_token=#{labels.fetch('session_cookies.david')}", 'Content-Type'=>'application/json'))
  {digits:, bytes:, storable: bytes <= SudoMode::MAX_STORED_PARAMS_BYTES, status: browser.response.status}
end
puts JSON.pretty_generate(result)
warn "Rails casting follow-ups: #{result[:expiry].size} renders, #{result[:save_list].size} HTTP saves/lists, #{result[:sudo].size} sudo forms, #{result[:size].size} integer size boundaries"
