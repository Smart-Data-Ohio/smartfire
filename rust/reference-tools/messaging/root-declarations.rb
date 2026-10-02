require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActiveJob::Base.queue_adapter=:test
ActionController::Base.allow_forgery_protection=false
Rails.application.config.hosts.clear
user=User.find(127326141);room=Room.find(486777696)
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
headers={'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
inputs=[{markdown_source:'stays the same',client_message_id:'declarations-identical'},
 {body:'<div>legacy text</div>',client_message_id:'declarations-rich'},
 {client_message_id:'declarations-bodyless'},
 {markdown_source:'UTC marker',client_message_id:'declarations-tz'},
 {markdown_source:'source',client_message_id:'declarations-source'},
 {markdown_source:'reply',client_message_id:'declarations-reply'},
 {markdown_source:'see https://github.com/rails/rails/pull/530',client_message_id:'declarations-fetch'}]
messages=inputs.map{|input|room.messages.create!(input.merge(creator:user))}
messages[2].attachment.attach(ActiveStorage::Blob.find(13))
messages[3].update_column(:edited_at,Time.utc(2026,9,22,12))
messages[5].update!(reply_to_message:messages[4])
rows=[]
[ ['same',0,{markdown_source:'stays the same'}],
 ['attachment_only',0,{markdown_source:'stays the same',attachment:ActiveStorage::Blob.find(13).signed_id}],
 ['rich_same',1,{body:messages[1].body.body.to_html}],
 ['bodyless_blank',2,{body:'',attachment:ActiveStorage::Blob.find(13).signed_id}],
 ['rich_format',1,{body:'<div><strong>legacy text</strong></div>'}]
].each_with_index do |(name,index,input),position|
 travel_to Time.utc(2026,3,2,16)+(position+1)*10
 message=messages[index]
 browser.patch("/rooms/#{room.id}/messages/#{message.id}.json",params:{message:input},headers:headers,as: :json)
 message.reload
 rows << {name:name,index:index,input:input,time:Time.current.iso8601,status:browser.response.status,body:browser.response.body,
  edited_at:message.edited_at&.utc&.iso8601(3),updated_at:message.updated_at.utc.iso8601(3),saved_body:message.body.body&.to_html.to_s}
end
Boost.create!(message:messages[0],booster:User.find(149087659),content:'👍')
reaction_edited=messages[0].reload.edited_at
pr=messages[6].github_pull_requests.sole
pr.update!(private:false,title:'Fetched',fetched_at:Time.current)
fetch_edited=messages[6].reload.edited_at
browser.delete("/rooms/#{room.id}/messages/#{messages[4].id}.turbo_stream",headers:headers)
reply_edited=messages[5].reload.edited_at
# No masking: compare the complete owned meta component in both real viewer time zones.
meta=[ 'Pacific Time (US & Canada)','Tokyo' ].map do |zone|
 user.update!(time_zone:zone)
 browser.get("/rooms/#{room.id}/messages/#{messages[3].id}",headers:headers)
 html=ApplicationController.renderer.new(http_host:'campfire.test',https:false).render(partial:'messages/meta',locals:{message:messages[3]})
 {zone:zone,status:browser.response.status,html:html}
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',inputs:inputs,message_ids:messages.map(&:id),rows:rows,
 reaction_edited:reaction_edited,fetch_edited:fetch_edited,reply_edited:reply_edited,meta:meta)+"\n")
puts 'WS8bm root declarations: 5 actual advancing-clock saves with exact response/row fields; reaction/fetch/tombstone no-edit facts; UTC meta in two actual viewer zones'
