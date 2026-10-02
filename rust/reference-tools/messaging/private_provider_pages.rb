require 'json'
require 'stringio'
load File.join(__dir__, 'providers.rb')
base = JSON.parse(File.read(ARGV.fetch(0)))
user = User.find(127326141)
room = Room.find(699448326)
ActionController::Base.allow_forgery_protection = false
Rails.application.config.hosts.clear
request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test', 'rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token] = user.sessions.where.not(two_factor_verified_at:nil).first!.token
headers = {'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
renderer = ApplicationController.renderer.new(http_host:'campfire.test', https:false)
pages = []
{'private'=>'https://github.com/provider-owner/repo-9/pull/10', 'unknown'=>'https://github.com/provider-owner/repo-10/pull/11', 'mixed'=>'https://github.com/provider-owner/repo-9/pull/10 https://github.com/provider-owner/repo-1/pull/2'}.each do |kind, links|
 messages = []
 [4,16].each do |size|
  Rails.application.executor.wrap do
   Current.reset
   Current.user = user
   (messages.size...size).each do |i|
    messages << room.messages.create!(creator:user, client_message_id:"cost-#{kind}-#{i}", markdown_source:"#{kind}onlyquery #{links}")
   end
  end
  path = "/searches?q=#{kind}onlyquery"
  browser.get(path, headers:headers.dup)
  queries=[]
  subscriber=->(*args){p=args.last; queries << p[:sql] if !p[:cached] && p[:name]!='SCHEMA' && p[:sql].match?(/\A\s*(SELECT|WITH)\b/i)}
  ActiveSupport::Notifications.subscribed(subscriber,'sql.active_record'){browser.get(path,headers:headers.dup)}
  raise 'search failed' unless browser.response.successful?
  cards=Rails.application.executor.wrap do
   Current.reset
   messages.map{|m| {id:m.id, html:renderer.render(partial:'github/pull_requests/cards',locals:{message:Message.with_rendering_details.find(m.id)})}}
  end
  pages << {kind:kind, size:size, links:links, reads:queries.size, cards:cards}
 end
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',rows:base['rows'],pages:pages)+"\n")
puts "WS8bm2 private provider Rails: #{pages.map{|p| "#{p[:kind]} #{p[:size]}=#{p[:reads]} reads"}.join('; ')}; #{pages.sum{|p|p[:cards].size}} card containers"
