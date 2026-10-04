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
strings = [nil, '', ' ', 'invalid', '2030-06-15', '20300615', '30-06-15', '15/06/2030', '06/15/2030',
 'June 15, 2030 10:20:30', '15 June 2030 10:20:30', 'Jun 2030', 'June 15', '15 Jun', 'June',
 '10:20:30', '10:20', '4pm', 'Fri', 'junk', '2030-166', '2030-W24-6', '2030W246',
 '20300615T102030Z', '2030/06/15 10:20:30', '2030.06.15 10:20:30', 'Sat, 15 Jun 2030 10:20:30 GMT',
 'Sat Jun 15 10:20:30 2030', '2030-06-15T10:20:30Z', '2030-06-15T10:20:30+05:30',
 '2030-06-15 10:20:30.123456789', '2030-02-30 10:20:30', '2030-06-15 24:00:00',
 '2030-06-15 10:20:60', '2030-06-15 10:20:30 EST', '2030-06-15 10:20:30 +0530',
 '2026-03-08T02:30:00', '2026-11-01T01:30:00', 12, 12.5, -12, 0, true, false, [], ['2030-06-15'], {'year'=>2030}]
multiparts = [
 {'expires_at(1i)'=>'2030','expires_at(2i)'=>'6','expires_at(3i)'=>'15','expires_at(4i)'=>'10','expires_at(5i)'=>'20'},
 {'expires_at(1i)'=>'2030','expires_at(2i)'=>'6','expires_at(3i)'=>'15'},
 {'expires_at(1i)'=>'2030','expires_at(2i)'=>'2','expires_at(3i)'=>'30'},
 {'expires_at(1i)'=>'2030','expires_at(2i)'=>'13','expires_at(3i)'=>'15'},
 {'expires_at(1i)'=>'','expires_at(2i)'=>'','expires_at(3i)'=>''},
 {'expires_at(1i)'=>'2030','expires_at(2i)'=>'','expires_at(3i)'=>'15'},
 {'expires_at(1i)'=>'2030','expires_at(2i)'=>'6'},
 {'expires_at'=>'2035-01-01','expires_at(1i)'=>'2030','expires_at(2i)'=>'6','expires_at(3i)'=>'15'},
 {'expires_at(1i)'=>true,'expires_at(2i)'=>6,'expires_at(3i)'=>15},
 {'expires_at(1i)'=>'2030x','expires_at(2i)'=>'6','expires_at(3i)'=>'15','expires_at(4i)'=>'25'},
 {'expires_at(1i)'=>'2030','expires_at(2i)'=>'6','expires_at(3i)'=>'15','expires_at(6f)'=>'30.75'},
 {'expires_at(1i)'=>'2026','expires_at(2i)'=>'11','expires_at(3i)'=>'1','expires_at(4i)'=>'1','expires_at(5i)'=>'30'}]
['UTC','America/New_York'].each do |zone|
 owner.update_columns(time_zone: zone)
 (strings.map{|value| {'expires_at'=>value}} + multiparts + [12,12.5,-12,0,true].map{|value| {'expires_at'=>value,'name'=>''}}).each_with_index do |attributes,index|
  name = "Input #{zone} #{index}"
  audits = AuditLog.where(action:'agent.credential.create').count
  browser.post("/account/bots/#{bot.id}/credentials", params: JSON.generate(agent_credential: {'name'=>name}.merge(attributes)), headers: headers.merge('Content-Type'=>'application/json'))
  if browser.response.status >= 400
   result[:errors][browser.response.status.to_s] = {body:browser.response.body,content_type:browser.response.headers['Content-Type']}
  end
  # Querying through the model would cast a numeric SQLite value back to nil.
  row = connection.select_one("SELECT expires_at FROM agent_credentials WHERE name=#{connection.quote(name)}")
  result[:expiry] << {zone:, attributes:, status: browser.response.status, persisted: !row.nil?, stored: row && row['expires_at'], audits: AuditLog.where(action:'agent.credential.create').count-audits}
 end
end
owner.update_columns(time_zone: 'UTC')
module TokenCoercionProbe
 class << self; attr_accessor :token; end
end
Github::WriteClient.singleton_class.prepend(Module.new do
 define_method(:authenticated_login) do |token|
  TokenCoercionProbe.token = token
  'fixture-machine'
 end
end)
tokens = [nil, '', " \t 
 ", false, true, 12, 12.5, [], ['first','second'], ['line\nquote"','é',true,12], [['nested'],{'x'=>'value'}], {}, {'value'=>'token'}, {'nested'=>{'x'=>'value'}, 'list'=>['one',true]}]
tokens += [
 ["line\nquote\"", "\x00\x01\x1f\x7f", '#{value} #@value #$value', "\u0085\u2028"],
 [{'nested'=>{'x'=>'value'},'list'=>[{'a'=>'b'}]}],
 {'list'=>[{'a'=>'b'}]},
 [[{'nested'=>{'x'=>'value'}}]]
]
tokens.each do |input|
 browser.get('/agents',headers:)
 TokenCoercionProbe.token = nil
 browser.post("/account/bots/#{bot.id}/github_connection", params: JSON.generate(access_token: input), headers: headers.merge('Content-Type'=>'application/json'))
 result[:github_tokens] << {input:, token: TokenCoercionProbe.token, status: browser.response.status, flash: browser.request.flash.to_hash.slice('notice','alert')}
end
upload = Rack::Test::UploadedFile.new(StringIO.new('file-token'), 'text/plain', original_filename:'token.txt')
browser.get('/agents',headers:)
TokenCoercionProbe.token = nil
browser.post("/account/bots/#{bot.id}/github_connection", params:{access_token:upload}, headers:)
raise 'uploaded file did not reach login' unless TokenCoercionProbe.token.match?(/\A#<ActionDispatch::Http::UploadedFile:0x[0-9a-f]+>\z/)
result[:github_upload] = {token_pattern:'^#<ActionDispatch::Http::UploadedFile:0x[0-9a-f]+>$', status:browser.response.status, flash:browser.request.flash.to_hash.slice('notice','alert')}
puts JSON.pretty_generate(result)
warn "Rails input boundaries: #{result[:expiry].size} credential HTTP cases; #{result[:github_tokens].size} token shapes; 1 uploaded file"
