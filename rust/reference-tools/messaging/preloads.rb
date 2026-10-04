# Full cached message bodies rendered by pinned Rails, with its rendering/mention preloads.
require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
user = User.find_by!(email_address:'david@37signals.com')
jason = User.find_by!(email_address:'jason@37signals.com')
bot = User.find(394959859)
room = user.rooms.find_by!(name:'All Talk')
Current.user = user
room.update_columns(icon_name:'github')
bot.update_columns(icon_name:'github')
source = room.messages.create!(creator:jason, markdown_source:'Source @[David]',client_message_id:'preload-source')
messages = []
messages << room.messages.create!(creator:user,markdown_source:'Markdown @[Jason] :github: **bold**',client_message_id:'preload-markdown')
messages << room.messages.create!(creator:user,body:"<div>Legacy <action-text-attachment sgid=\"#{jason.attachable_sgid}\"></action-text-attachment></div>",client_message_id:'preload-legacy')
messages << room.messages.create!(creator:bot,markdown_source:'Bot note',client_message_id:'preload-bot')
messages << room.messages.create!(creator:user,markdown_source:'Reply text',reply_to_message:source,client_message_id:'preload-reply')
messages << room.messages.create!(creator:user,markdown_source:'Poll question?',client_message_id:'preload-poll')
poll=Poll.create_for_message!(message:messages.last,labels:['One','Two'],multiple:true,anonymous:false,closes_at:1.day.from_now)
poll.cast_vote!(jason,[poll.poll_options.first.id])
MessagePin.create!(message:messages.last,room:,pinner:user)
messages.first.boosts.create!(booster:jason,content:':github:')
messages.first.boosts.create!(booster:bot,content:'🙌')
messages << room.messages.create!(creator:user,body:'',client_message_id:'preload-file')
blob=ActiveStorage::Blob.create!(key:'preload-file-blob',filename:'report & data.txt',content_type:'text/plain',metadata:{},service_name:'local',byte_size:12,checksum:'fixture-checksum')
ActiveStorage::Attachment.create!(record:messages.last,name:'attachment',blob:)
messages=Message::MentionPreloader.preload_for(Message.with_rendering_details.where(id:messages.map(&:id)).ordered.to_a)
renderer=ApplicationController.renderer.new(http_host:'campfire.test',https:false)
html=messages.map do |message|
 Rails.cache.clear
 {id:message.id,html:renderer.render(partial:'messages/message',locals:{message:,show_room_icon:true})}
end
all_ids=messages.map(&:id)+[source.id]
ids=all_ids.join(',')
poll_ids=poll.id
selects={
 'messages'=>"id IN (#{ids})",'action_text_rich_texts'=>"record_type='Message' AND record_id IN (#{ids})",'boosts'=>"message_id IN (#{ids})",'polls'=>"message_id IN (#{ids})",'poll_options'=>"poll_id=#{poll_ids}",'poll_votes'=>"poll_id=#{poll_ids}",'message_pins'=>"message_id IN (#{ids})",'active_storage_blobs'=>"id=#{blob.id}",'active_storage_attachments'=>"record_type='Message' AND record_id IN (#{ids})"
}
rows=selects.to_h{|table,where|[table,ActiveRecord::Base.connection.select_all("SELECT * FROM #{table} WHERE #{where} ORDER BY id").to_a]}
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],rows:,room_id:room.id,bot_id:bot.id,html:)+"\n")
puts "WS8bm2 preload Rails oracle: #{html.size} complete message fragments; #{rows.size} committed fixture tables"
