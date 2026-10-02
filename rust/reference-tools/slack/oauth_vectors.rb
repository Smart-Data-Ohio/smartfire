require 'json'
require 'net/http'
# The real Rails OAuth methods run against transport inputs only. No outbound socket.
class Ws16OAuthTransport
  attr_accessor :input, :requests
  def post(path, body, headers)
    answer('POST', path, URI.decode_www_form(body).to_h, headers)
  end
  def request(request)
    answer(request.method, request.path, nil, request.to_hash.transform_values(&:first))
  end
  def answer(method, path, form, headers)
    requests << { method:, path:, form:, accept: headers['Accept'] || headers['accept'],
      content_type: headers['Content-Type'] || headers['content-type'],
      authorization_matches: (headers['authorization'] == ['Bearer', 'fixture-user-grant'].join(' ')) }
    if input['exception']
      raise Object.const_get(input['exception']), 'fixture detail must never leak'
    end
    response = Net::HTTPResponse::CODE_TO_OBJ.fetch(input['status'].to_s).new('1.1', input['status'].to_s, 'Fixture')
    response.instance_variable_set(:@read, true)
    response.body = input['body']
    response
  end
end
transport = Ws16OAuthTransport.new
Net::HTTP.singleton_class.prepend(Module.new do
  define_method(:start) do |host, port, **timeouts, &block|
    raise 'Unexpected OAuth endpoint' unless host == 'slack.com' && port == 443
    raise 'Unexpected OAuth deadline' unless timeouts == { use_ssl: true, open_timeout: 10, read_timeout: 10, write_timeout: 10 }
    block.call(transport)
  end
end)
base = { 'ok' => true, 'authed_user' => { 'id' => 'UFIXTURE', 'access_token' => 'fixture-user-grant', 'scope' => Slack::OAuth::USER_SCOPES.join(',') }, 'team' => { 'id' => 'TFIXTURE', 'name' => 'Fixture' } }
inputs = [base, {}, { 'ok' => false, 'error' => 'invalid_code' }, { 'ok' => false, 'error' => '' }, { 'ok' => false, 'error' => ['a', 'b'] },
  { 'ok' => true }, { 'ok' => true, 'authed_user' => nil }, { 'ok' => true, 'authed_user' => [] }, { 'ok' => true, 'authed_user' => false },
  *[nil, '', " \n", false, 0, [], {}, 'fixture-user-grant'].map { |v| { 'ok' => true, 'authed_user' => { 'access_token' => v } } },
  nil, [], false, 5, 'ok', 'error', 'ok team', { 'ok' => 0, 'authed_user' => { 'access_token' => 'fixture-user-grant' } },
  { 'ok' => '', 'authed_user' => { 'access_token' => 'fixture-user-grant' } },
  { 'revoked' => true }, { 'revoked' => 1 }, { 'ok' => true, 'team' => { 'domain' => 'fixture' } }].map { |b| { 'status' => 200, 'body' => JSON.generate(b) } }
inputs += [{ 'status' => 201, 'body' => JSON.generate(base) }, { 'status' => 401, 'body' => JSON.generate(base) },
 { 'status' => 500, 'body' => JSON.generate({ 'revoked' => true, 'ok' => true, 'team' => { 'id' => 'TFIXTURE' } }) },
 { 'status' => 200, 'body' => '{bad' }, { 'status' => 200, 'body' => '' }]
%w[Net::OpenTimeout Net::ReadTimeout Net::WriteTimeout SocketError OpenSSL::SSL::SSLError EOFError Errno::ECONNREFUSED].each { |e| inputs << { 'exception' => e } }
cases = inputs.map do |input|
  transport.input = input
  transport.requests = []
  expected = {}
  begin
    expected['exchange'] = { 'body' => Slack::OAuth.exchange_code(client_id: 'fixture-client', client_secret: 'fixture-secret', code: 'fixture-code', redirect_uri: 'https://example.invalid/slack/oauth/callback') }
  rescue Slack::OAuth::Error => error
    expected['exchange'] = { 'error' => error.message }
  end
  expected['revoke'] = Slack::OAuth.revoke('fixture-user-grant')
  expected['team_info'] = Slack::OAuth.team_info('fixture-user-grant')
  { 'input' => input, 'expected' => expected, 'requests' => transport.requests }
end
urls = [nil, '', ' ', 'TFIXTURE'].map { |team| { 'team' => team, 'url' => Slack::OAuth.authorize_url(client_id: 'client +&', redirect_uri: 'https://example.invalid/slack/oauth/callback', state: 'raw +state', team_id: team) } }
verifier = Rails.application.message_verifier('slack_oauth_state')
result = { 'reference' => 'd7c7de9264c63015be398001d7a1094e7695a6db', 'cases' => cases, 'authorize' => urls,
 'manifest' => Slack::AppManifest.to_json(base_url: 'https://example.invalid'),
 'state' => { 'raw' => 'fixture-state', 'signed' => verifier.generate('fixture-state'), 'secret_key_base' => ENV.fetch('SECRET_KEY_BASE') } }
File.write(File.join(ENV.fetch('PARITY_WORK'), 'vectors/slack/oauth.json'), JSON.pretty_generate(result) + "\n")
puts "Slack OAuth vectors: #{cases.length} real Rails exchange/revoke/team cases, 4 authorization URLs, manifest and signed state generated"
