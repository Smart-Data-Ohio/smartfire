# Pinning Google::Client's requests, responses and errors from our Rails app.
require "json"
require "net/http"
load File.join(ENV.fetch("PARITY_WORK"), "reference-tools/google/google_calendar_test_helper.rb")
helper = Object.new.extend(GoogleCalendarTestHelper)
ENV["GOOGLE_CLIENT_ID"] = "test-client-id"
ENV["GOOGLE_CLIENT_SECRET"] = "FAKE-google-client-secret"
now = Time.utc(2026, 9, 30, 12)
# Run these vectors at a fixed instant regardless of the reference seed time.
Time.define_singleton_method(:current) { now }
http = Object.new
calls = []
responses = []
http.define_singleton_method(:method_missing) do |method, path, *args|
  status, body = responses.shift || raise("unexpected Google request")
  headers = args.last.is_a?(Hash) ? args.last : {}
  calls << { method: method.to_s.upcase, path:, body: %i[get delete].include?(method) ? "" : args.first.to_s,
    content_type: headers["Content-Type"], access_token: headers["Authorization"]&.delete_prefix("Bearer ") }
  response = Net::HTTPResponse::CODE_TO_OBJ.fetch(status.to_s).new("1.1", status.to_s, "fixture")
  response.define_singleton_method(:body) { body }
  response
end
Net::HTTP.define_singleton_method(:start) { |*args, **kwargs, &block| block.call(http) }
refresh = {access_token: "refreshed-access-token", expires_in: 3600, token_type: "Bearer"}
invalid = {error: "invalid_grant", error_description: "Token has been expired or revoked."}
file = helper.send(:drive_file_payload)
list = helper.send(:drive_list_payload)
result = { reference: ENV.fetch("PARITY_REFERENCE_SHA"), now: now.to_i,
  authorize: [false,true].map { |drive| Google::Client.authorize_url(redirect_uri: "http://test.host/google/callback",state:"signed-state",drive:) },
  refresh:, invalid_grant: invalid, drive_file: file, drive_list: list, scenarios: [] }
[
  ["insert", [[200,{id:"abc123"}.to_json]], :insert_event, [{summary:"Party"}]],
  ["expired", [[200,refresh.to_json],[200,{id:"abc123"}.to_json]], :insert_event, [{summary:"Party"}]],
  ["401", [[401,""],[200,refresh.to_json],[200,"{}"]], :insert_event, [{}]],
  ["invalid_grant", [[400,invalid.to_json]], :insert_event, [{}]],
  ["drive_file", [[200,file.to_json]], :drive_file, [file["id"]]],
  ["drive_search", [[200,list.to_json]], :list_drive_files, [{query:"bob's\\draft"}]],
  ["drive_forbidden", [[403,"{}"]], :drive_file, [file["id"]]],
  ["calendar_quota", [[403,helper.send(:google_forbidden_body).to_json]], :insert_event, [{}]],
  ["calendar_forbidden", [[403,helper.send(:google_forbidden_body,"forbidden").to_json]], :insert_event, [{}]],
  ["malformed", [[200,"{oops"]], :insert_event, [{}]],
  ["pages", [[200,{items:[{id:"one"}],nextPageToken:"token-2"}.to_json],[200,{items:[{id:"two"}]}.to_json]], :list_events, [{time_min:now-3600,time_max:now+3600}]]
].each do |name, answers, method, args|
  responses.replace(answers.map(&:dup)); calls.clear
  credentials = Google::Client::SnapshotCredentials.new(access_token:"access-token",refresh_token:"refresh-token",access_token_expires_at: %w[expired invalid_grant].include?(name) ? now-3600 : now+3600)
  client = Google::Client.new(credentials)
  begin
    value = args.last.is_a?(Hash) && %i[list_drive_files list_events].include?(method) ? client.public_send(method,**args.last) : client.public_send(method,*args)
    error = nil
  rescue Google::Client::Error => e
    value=nil; error={class:e.class.name,message:e.message}
  end
  result[:scenarios] << {name:,responses:answers,requests:calls.dup,result:value,error:}
end
result[:email_tokens] = [
  {}, {iss:"accounts.google.com"}, {iss:"evil"}, {aud:"other"}, {exp:now.to_i}, {exp:now.to_i-3600}, {email:nil}, {email:" "}, {exp:"#{now.to_i+3600}suffix"}
].map do |overrides|
  token=helper.send(:google_id_token,**{exp:now.to_i+3600}.merge(overrides))
  begin
    {token:,email:Google::Client.email_from_id_token(token),error:nil}
  rescue Google::Client::Error=>e
    {token:,email:nil,error:e.message}
  end
end
result[:email_tokens].concat([nil, '', 'not-a-jwt'].map do |token|
  begin
    {token:,email:Google::Client.email_from_id_token(token),error:nil}
  rescue Google::Client::Error=>e
    {token:,email:nil,error:e.message}
  end
end)
puts JSON.pretty_generate(result)
