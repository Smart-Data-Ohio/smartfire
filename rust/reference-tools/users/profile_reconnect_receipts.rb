# Exact original missing-account and refresh-error setups from ProfilesControllerTest.
require 'json'
require 'digest'
require 'cgi'
JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'reference-tools/users/profiles-source-hashes.json'))).each do |path, hash|
  raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
ENV['GOOGLE_CLIENT_ID'] = 'parity-client'
ENV['GOOGLE_CLIENT_SECRET'] = 'parity-secret'
Rails.application.config.hosts.clear
Rails.logger = ActiveSupport::Logger.new($stderr)
user = User.find(127326141)
client = ActionDispatch::Integration::Session.new(Rails.application)
client.host! 'campfire.test'
request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST' => 'campfire.test', 'rack.input' => StringIO.new))
request.cookie_jar.signed.permanent[:session_token] = {value: user.sessions.first.token, httponly: true, same_site: :lax}
client.cookies['session_token'] = request.cookie_jar[:session_token]
cases = %w[missing_account fetch_error].map do |name|
  GoogleAccount.where(user: user).delete_all
  Calendar::MeetingCache.where(user: user).delete_all
  user.update_columns(meeting_status_enabled: true, ooo_calendar_enabled: false)
  if name == 'fetch_error'
    GoogleAccount.create!(user: user, email: 'david@gmail.test')
    Calendar::MeetingCache.create!(user: user, fetched_at: Time.current, fetch_error: Calendar::MeetingRefresh::UNREACHABLE_MESSAGE)
  end
  client.get '/users/me/profile'
  ActiveSupport::IsolatedExecutionState.clear
  doc = Nokogiri::HTML(client.response.body)
  error = user.reload.meeting_cache&.fetch_error
  {name: name, account_exists: GoogleAccount.exists?(user: user), fetch_error: error,
   response: {status: client.response.status,
    reconnect_links: doc.css("a[href='#google-calendar-title']").select { |a| a.text == 'Reconnect below' }.map { |a| [a['href'], a.text] },
    escaped_fetch_error: error && CGI.escapeHTML(error)}}
end
puts JSON.pretty_generate(reference: ENV.fetch('PARITY_REFERENCE_SHA'), cases: cases)
warn "Rails original reconnect receipts: #{cases.size} exact original profile GET setups; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
