# Real shared message fragments and successful forms under forgery protection.
require 'json'
require 'nokogiri'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActiveJob::Base.queue_adapter = :test
ActionController::Base.allow_forgery_protection = true
Rails.cache = ActiveSupport::Cache::MemoryStore.new
ActionView::PartialRenderer.collection_cache = Rails.cache
ActionController::Base.perform_caching = true
Rails.application.config.hosts.clear
users=[User.find(127326141),User.find(149087659)]
room=Room.find(486777696)
pr=Github::PullRequest.for_reference(owner:'rails',repo:'rails',number:3141)
pr.update!(private:false,title:'Cached card',state:'open',fetched_at:Time.current)
pr.update_column(:fetch_requested_at,nil)
card=room.messages.create!(creator:users[0],markdown_source:'see https://github.com/rails/rails/pull/3141',client_message_id:'csrf-pr')
poll_message=room.root_messages.create!(creator:users[0],markdown_source:'Lunch?',client_message_id:'csrf-poll')
poll=Poll.create_for_message!(message:poll_message,labels:['Tacos','Pizza'])
boosted=room.messages.create!(creator:users[0],markdown_source:'Boost me',client_message_id:'csrf-boosts')
boosted.boosts.create!(booster:users[0],content:'👍')
legacy=boosted.boosts.create!(booster:users[1],content:'Legacy text boost')
thread=ChannelThread.create!(room:room,creator:users[0],name:'Cached thread')
ThreadMembership.join!(thread,users[0])
reply=thread.post_message!(creator:users[0],attributes:{markdown_source:'In the thread',client_message_id:'csrf-thread'})
reply.boosts.create!(booster:users[0],content:'🎉')
browsers=users.map do |user|
 request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
 request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
 browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
 browser.get("/rooms/#{room.id}/threads/#{thread.id}",headers:{'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"})
 browser
end
rows=[]
["/rooms/#{room.id}/messages", "/rooms/#{room.id}/refresh.turbo_stream?since=#{(Time.current-60).to_fs(:epoch)}", "/rooms/#{room.id}/threads/#{thread.id}/messages"].each do |path|
 bodies=browsers.map do |browser|
  browser.get(path)
  raise "bad page #{browser.response.status}" unless browser.response.status==200
  doc=Nokogiri::HTML4(browser.response.body)
  ids=path.include?('/threads/') ? [reply.id] : [card.id,poll_message.id,boosted.id]
  ids.map{|id|doc.at_css("[data-message-id='#{id}']").to_html}
 end
 raise 'viewer dependent fragments' unless bodies[0]==bodies[1]
 raise 'cached form token' if bodies.flatten.any?{|html|html.include?('authenticity_token')}
 rows << {path:path,message_ids:path.include?('/threads/') ? [reply.id] : [card.id,poll_message.id,boosted.id]}
end
fragments=[card,poll_message,boosted,reply].map do |message|
 html=ApplicationController.renderer.new(http_host:'campfire.test',https:false).render(partial:'messages/message',collection:[message],cached:->(record){ApplicationController.helpers.message_with_pr_cards_cache_key(record)})
 {id:message.id,html:html}
end
# Do not claim poll or the room-shell header here: those are separate-owner integration.
browser=browsers[1]
browser.get("/rooms/#{room.id}/threads/#{thread.id}")
header=Nokogiri::HTML5(browser.response.body).at_css('meta[name=csrf-token]')['content']
forms=[]
[card,boosted,reply].each do |message|
 Current.user=users[1]
 html=ApplicationController.renderer.new(http_host:'campfire.test',https:false).render(partial:'messages/message',collection:[message],cached:->(record){ApplicationController.helpers.message_with_pr_cards_cache_key(record)})
 Nokogiri::HTML4(html).css('form').each do |form|
  params={};form.css('input[type=hidden]').each{|field|params[field['name']]=field['value']}
  method=params.delete('_method')||form['method']||'get'
  browser.process(method.to_sym,form['action'],params:params,headers:{'X-CSRF-Token'=>header,'Accept'=>'text/vnd.turbo-stream.html, text/html, application/xhtml+xml'})
  forms << {action:form['action'],method:method,params:params,status:browser.response.status}
  raise "form failure #{forms.last}" unless (200..303).include?(browser.response.status)
 end
end
Current.reset
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],fragments:fragments,poll_message_id:poll_message.id,poll_id:poll.id,poll_options:poll.poll_options.pluck(:id,:label,:position),card_id:card.id,boosted_id:boosted.id,legacy_boost_id:legacy.id,thread_id:thread.id,reply_id:reply.id,rows:rows,forms:forms)+"\n")
puts "WS8bm cached CSRF: 6 actual two-viewer pages; #{forms.size} successful reaction/legacy/GitHub forms with the page header; polls and room-shell header separately owned"
