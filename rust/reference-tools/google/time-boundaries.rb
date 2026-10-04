# Exact equality and adjacent instants from pinned Rails, with recorded HTTP only.
require 'json'
require 'net/http'
ENV['GOOGLE_CLIENT_ID'] = 'test-client-id'
ENV['GOOGLE_CLIENT_SECRET'] = 'FAKE-google-client-secret'
now = Time.utc(2026, 9, 30, 12)
Time.define_singleton_method(:current) { now }
http = Object.new
responses = []
calls = []
http.define_singleton_method(:method_missing) do |method, path, *args|
  status, body = responses.shift || raise('unrecorded Google HTTP forbidden')
  calls << {method: method.to_s.upcase, path:}
  response = Net::HTTPResponse::CODE_TO_OBJ.fetch(status.to_s).new('1.1',status.to_s,'fixture')
  response.define_singleton_method(:body) { body }
  response
end
Net::HTTP.define_singleton_method(:start) { |*args, **kwargs, &block| block.call(http) }
result = {reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], now: now.to_i}
result[:access] = [-1,0,1].map do |microseconds|
  expiry = now + Rational(microseconds,1_000_000)
  account = GoogleAccount.new(access_token: 'access-token',access_token_expires_at: expiry)
  snapshot = Google::Client::SnapshotCredentials.new(access_token:'access-token',refresh_token:'refresh-token',access_token_expires_at:expiry)
  expired = snapshot.access_token_expired?
  answers = expired ? [[200,{access_token:'refreshed-access-token',expires_in:3600}.to_json],[200,'{}']] : [[200,'{}']]
  responses.replace(answers.map(&:dup)); calls.clear
  Google::Client.new(snapshot).get_event('boundary')
  {microseconds:,expired:account.access_token_expired?,snapshot_expired:expired,responses:answers,requests:calls.dup}
end
result[:flow] = [-1,0,1].map do |offset|
  flow = {'state'=>'fixture-state','nonce'=>'fixture-nonce','verifier'=>'fixture-verifier','exp'=>now.to_i+offset}
  {offset:,valid:Sessions::GoogleController.new.send(:valid_flow?,flow,'fixture-state')}
end
result[:renewal] = [-1,0,1].map do |microseconds|
  expires_at = now + 24.hours + Rational(microseconds,1_000_000)
  # Run the production sweep and observe whether it invokes renewal.
  channel = Calendar::PushChannel.find_or_initialize_by(user_id:127326141)
  channel.assign_attributes(channel_id:'fixture-boundary',token_digest:Calendar::PushChannel.digest('fixture-token'),expires_at:)
  channel.save!
  user = User.find(127326141)
  GoogleAccount.where(user:).delete_all
  GoogleAccount.create!(user:,email:'fixture@example.test',access_token:'access-token',refresh_token:'refresh-token',access_token_expires_at:now+1.hour)
  ENV['GOOGLE_CALENDAR_WEBHOOK_URL']='https://campfire.test/google/calendar/notifications'
  renewed = false
  Calendar::PushChannel.define_method(:renew!) { renewed = true }
  Calendar::PushChannel.define_singleton_method(:heal_missing_channels!) {}
  Calendar::PushChannel.renew_expiring!(now:)
  {microseconds:,renewed:}
end
result[:cache] = [-1,0,1].map do |microseconds|
  user = User.find(127326141)
  user.update_columns(meeting_status_enabled:true)
  cache = Calendar::MeetingCache.find_or_initialize_by(user:)
  cache.assign_attributes(fetched_at:now-60+Rational(microseconds,1_000_000),refresh_pending_at:now)
  cache.save!
  responses.replace([[200,'{"items":[]}']]);calls.clear
  {microseconds:,result:Calendar::MeetingRefresh.refresh(user.id,now:).to_s,requests:calls.dup}
end
result[:followup] = [-1,0,1].map do |microseconds|
  cache = Calendar::MeetingCache.find_by!(user_id:127326141)
  cache.update_columns(refresh_pending_at:now-60+Rational(microseconds,1_000_000))
  {microseconds:,claimed:cache.claim_refresh_followup!(now:)}
end
puts JSON.pretty_generate(result)
