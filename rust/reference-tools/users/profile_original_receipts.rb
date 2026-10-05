# Per-assertion additions for the original ProfilesControllerTest, d7c7de92.
# Existing settings, security, Calendar and sound-window corpora cover the other clauses.
require 'json'
require 'digest'
JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'reference-tools/users/profiles-source-hashes.json'))).each do |path, hash|
  raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
load File.join(ENV.fetch('PARITY_WORK'), 'reference-tools/users/post_pin.rb')
Rails.application.config.hosts.clear
Rails.logger = ActiveSupport::Logger.new($stderr)
ENV.delete('GOOGLE_CLIENT_ID')
ENV.delete('GOOGLE_CLIENT_SECRET')
user = User.find(127326141)
GoogleAccount.where(user: user).delete_all
user.update_columns(inbox_preferences: nil, voice_mode: nil, push_to_talk_key: nil, time_zone: nil)
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST' => 'campfire.test', 'rack.input' => StringIO.new))
request.cookie_jar.signed.permanent[:session_token] = {value: user.sessions.first.token, httponly: true, same_site: :lax}
browser.cookies['session_token'] = request.cookie_jar[:session_token]
def observe(browser)
  doc = Nokogiri::HTML(browser.response.body)
  {
    status: browser.response.status,
    inbox: doc.css('input[type=checkbox]').filter_map { |n| n['name'] if n['name'].start_with?('user[inbox_preferences]') && n['checked'] },
    notification_explanations: ['GitHub review requests', 'The incoming-call banner still shows.'].map { |s| browser.response.body.include?(s) },
    voice_mode: doc.css('select[name="user[voice_mode]"] option[selected]').map { |n| [n['value'], n.text] },
    push_to_talk_key: doc.css('input[name="user[push_to_talk_key]"]').map { |n| n['value'] },
    current_password: doc.css('input[name="user[current_password]"]').map { |n| n['autocomplete'] },
    time_zone: doc.css('select#user_time_zone option[selected]').map { |n| n['value'] },
    meeting_dnd_checkbox: doc.css('input[name="user[meeting_dnd_enabled]"][type=checkbox]').size,
    meeting_dnd_explanation: browser.response.body.include?('Do not disturb during meetings'),
    calendar_not_configured: browser.response.body.include?('Google Calendar is not configured for this workspace'),
    calendar_connect: browser.response.body.include?('Connect Google Calendar'),
    drive_row: ['Enable Drive previews', 'Drive previews enabled'].any? { |s| browser.response.body.include?(s) }
  }
end
pages = [nil, 'Pacific Time (US & Canada)', 'America/New_York'].map do |zone|
  user.update_columns(time_zone: zone)
  browser.get '/users/me/profile'
  ActiveSupport::IsolatedExecutionState.clear
  {zone: zone, response: observe(browser)}
end
csrf = Nokogiri::HTML(browser.response.body).at_css('meta[name="csrf-token"]')['content']
mutations = [
  ['original_text_size', {text_size: 'smaller'}],
  ['original_notifications', {inbox_preferences: {github_review_requests: '0', agent_approvals: '0', agent_work: '1', event_reminders: 'false', huddle_invitations: 'true'}}],
  ['original_bad_notification', {inbox_preferences: {github_review_requests: 'banana'}}],
  ['original_bad_voice', {voice_mode: 'shout'}],
  ['original_github_duplicate', {github_login: 'Shared-Login'}]
].map do |name, params|
  user.update_columns(text_size: 'default', voice_mode: 'voice_activity', inbox_preferences: nil, github_login: nil)
  User.find(149087659).update_columns(github_login: 'shared-login')
  browser.put '/users/me/profile', params: {user: params}, as: :json, headers: {'X-CSRF-Token' => csrf, 'Accept' => 'text/html'}
  ActiveSupport::IsolatedExecutionState.clear
  {name: name, params: params, status: browser.response.status, location: browser.response.location,
   state: user.reload.attributes.slice('text_size', 'voice_mode', 'inbox_preferences', 'github_login'),
   duplicate_error: browser.response.body.include?('already linked to another user'),
   enabled_notifications: User::InboxPreferences::KEYS.filter_map { |key| "user[inbox_preferences][#{key}]" if user.inbox_preferences.public_send(key) }}
end
puts JSON.pretty_generate(reference: 'd7c7de92', pages: pages, mutations: mutations)
warn "Rails original profile receipts: #{pages.size} GETs; #{mutations.size} PUTs; 0 failures"
