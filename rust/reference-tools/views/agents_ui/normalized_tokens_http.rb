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

module ExtremeTokenProbe
 class << self; attr_accessor :token; end
end
Github::WriteClient.singleton_class.prepend(Module.new do
 define_method(:authenticated_login) do |token|
  ExtremeTokenProbe.token=token
  'fixture-machine'
 end
end)
raws=['1e309','-1e309','18446744073709551617','[18446744073709551617]','[null,1e309,null]']
paths=["/account/bots/#{bot.id}/github_connection", "/account/bots/#{bot.id}/github_connection/", "/account//bots/#{bot.id}//github_connection"]
paths.each do |path|
 raws.each do |raw|
  ExtremeTokenProbe.token=nil
  browser.post(path,params:'{"access_token":'+raw+'}',headers:headers.merge('Content-Type'=>'application/json'))
  result[:github_tokens] << {path:,raw:,token:ExtremeTokenProbe.token,status:browser.response.status}
 end
end
['1e309','-1e309','2e308','1e999999'].each do |number|
 raw='{"access_token":"valid-token","unused":'+number+'}'
 ExtremeTokenProbe.token=nil
 browser.post(paths.first,params:raw,headers:headers.merge('Content-Type'=>'application/json'))
 result[:github_tokens] << {path:paths.first,raw:'"valid-token"',body:raw,token:ExtremeTokenProbe.token,status:browser.response.status}
end
['0e309','-0e309','0e999','-0e999','0e2147483647','0e2147483648','0e9223372036854775807','0e9223372036854775808','0e99999999999999999999999','-0e99999999999999999999999'].each do |raw|
 ExtremeTokenProbe.token=nil
 browser.post(paths.first,params:'{"access_token":'+raw+'}',headers:headers.merge('Content-Type'=>'application/json'))
 result[:github_tokens] << {path:paths.first,raw:,token:ExtremeTokenProbe.token,status:browser.response.status}
end
puts JSON.pretty_generate(result)
warn "Rails normalized route token HTTP: #{result[:github_tokens].length} cases"
