require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
Rails.logger = ActiveSupport::TaggedLogging.new(Logger.new($stderr))
travel_to Time.utc(2026,3,2,16) do
  bot=User.find(394959859);room=Room.find(486777696);Current.user=bot
  key="#{bot.id}-BenderToken1"
  client=ActionDispatch::Integration::Session.new(Rails.application);client.host! "campfire.test";client.https!
  message=room.messages.create!(creator:bot,markdown_source:"**Old**")
  client.put "/rooms/#{room.id}/#{key}/messages/#{message.id}",params:"Fresh raw body",headers:{"CONTENT_TYPE"=>"text/plain"}
  message.reload
  edit={status:client.response.status,markdown_source:message.markdown_source,body:message.body.body.to_html,edited:message.edited_at.present?}
  boosts=[" :thumbsup: ",":heart:",":github:",":unknown_ws11:"," words ","\u00a0","🇺🇸",""].map do |content|
    client.post "/rooms/#{room.id}/#{key}/messages/#{message.id}/boosts",params:content,headers:{"CONTENT_TYPE"=>"text/plain"}
    {input:content,status:client.response.status,stored:client.response.status==201 ? Boost.order(:id).last.content : nil}
  end
  client_ids=[false,true,7,1.5,[],["x",nil],{}, {"k"=>"v"},"x",[["x"]],[[]],[{}],[{"k"=>"v"}],[true,false],["x","y"],"",['x#{}',"\n","\u001b"],1e20,1e-7,1e-4,1e-5,1e14,1e15,-0.0,[(0..127).map(&:chr).join],["é😀\u2028\u0085\u200b"]].map do |value|
    locations=[];statuses=[]
    2.times do
      client.post "/rooms/#{room.id}/#{key}/messages",params:JSON.generate(message:{client_message_id:value}),headers:{"CONTENT_TYPE"=>"application/json"}
      statuses<<client.response.status;locations<<client.response.headers["Location"]
    end
    stored=Message.find_by(id:locations.last&.split("/")&.last)&.client_message_id
    {input:value,statuses:statuses,replayed:locations[0] && locations[0]==locations[1],stored:stored}
  end
  puts JSON.pretty_generate(reference_pin:"d7c7de92",edit:edit,boosts:boosts,client_ids:client_ids)
end
