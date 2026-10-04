# Root POST triggers with Rails' real model callbacks and test queue.
require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActiveJob::Base.queue_adapter=:test
ActionController::Base.allow_forgery_protection=false
Rails.application.config.hosts.clear
user=User.find(127326141);room=Room.find(486777696);bot=User.find(394959859)
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
headers={'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
mention=room.messages.build(creator:user,markdown_source:'Hey @[Bender Bot]');mention.valid?
inputs=[{body:mention.body.to_s,client_message_id:'bot-declaration-rich'},{markdown_source:'Hey @[Bender Bot]',client_message_id:'bot-declaration-markdown'},{markdown_source:'Hey @[Bender Bot]',client_message_id:'agent-once'}]
rows=inputs.map do |input|
 ActiveJob::Base.queue_adapter.enqueued_jobs.clear
 browser.post("/rooms/#{room.id}/messages.turbo_stream",params:{message:input},headers:headers)
 jobs=ActiveJob::Base.queue_adapter.enqueued_jobs.map{|job|job[:job].name}.tally
 {input:input,status:browser.response.status,body:browser.response.body,delivery:jobs['Agent::DeliveryJob'].to_i,legacy:jobs['Bot::WebhookJob'].to_i}
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],agent_id:bot.agent.id,rows:rows)+"\n")
puts 'WS8bm bot controller declarations: 3 actual rich-text/Markdown root POSTs; exact response bytes and agent-only durable job counts'
