# Run with --time 2026-03-02T16:00:00Z --freeze: insert_all timestamps use SQLite's clock.
require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActionController::Base.allow_forgery_protection=false
user=User.find_by!(email_address:'david@37signals.com')
kevin=User.find_by!(email_address:'kevin@37signals.com')
Current.user=user
room=Rooms::Closed.create!(id:918001,name:'Files & plans',creator:user)
room.memberships.grant_to([user])
source_room=user.rooms.find_by!(name:'All Talk')
quote_room=user.rooms.find_by!(name:'Quiet Corner')
thread=room.channel_threads.create!(creator:user,name:'Files thread')
messages=[]; blobs=[]; uploads=[]; drives=[]
33.times do |i|
 names=['alpha-report.pdf','beta-mockup.png','movie.mp4','literal%_\\file.txt','document.docx','archive.zip','unknown.bin','notes & <draft>.txt','folder/data?.txt']
 types=['application/pdf','image/png','video/mp4','text/plain','application/vnd.openxmlformats-officedocument.wordprocessingml.document','application/zip',nil,'text/plain','text/plain']
 name=names[i]||"paged-file-#{i.to_s.rjust(2,'0')}.txt"
 type=i<types.size ? types[i] : 'text/plain'
 message=room.messages.create!(creator:user,markdown_source:name,client_message_id:"files-upload-#{i}",thread:(i==0 ? thread : nil))
 blob=ActiveStorage::Blob.create!(key:"files-blob-#{i}",filename:name,content_type:(type||'application/octet-stream'),metadata:{},service_name:'local',byte_size: i==0 ? 1663999 : i+1,checksum:'fixture-checksum')
 upload=ActiveStorage::Attachment.create!(record:message,name:'attachment',blob:,created_at:Time.current-i.minutes)
 drive=message.drive_attachments.create!(file_id:"picker0123456789#{i}",created_at:Time.current-i.minutes)
 blob.update_columns(content_type:nil) if type.nil?
 messages<<message; blobs<<blob; uploads<<upload; drives<<drive
end
source=source_room.messages.create!(creator:user,markdown_source:('Source @[Jason] & <private> '+('長'*210)),client_message_id:'quote-source')
quote=quote_room.messages.create!(creator:user,markdown_source:"http://campfire.test/rooms/#{source_room.id}/@#{source.id}",client_message_id:'quote-target')
ref=quote.message_references.first!
renderer=ApplicationController.renderer.new(http_host:'campfire.test',https:false)
card=renderer.render(partial:'messages/message_links/card',locals:{source:})
private_html=renderer.render(partial:'messages/message_links/private')
helper=ApplicationController.helpers
sizes=[0,1,2,512,1023,1024,1536,1048575,1048576,1663999,1073741824,1125899906842624,-1,-2048,999499,999500].map{|size|{size:,text:helper.number_to_human_size(size)}}
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
headers_for=->(u){request.cookie_jar.signed[:session_token]=u.sessions.where.not(two_factor_verified_at:nil).first!.token;{'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}}
headers=headers_for.call(user)
browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
paths=['', '?type=images','?type=videos','?type=documents','?type=other','?type=unknown','?filename=REPORT','?filename=%25','?filename=%5F','?filename=%5C','?filename=%20%20notes%20%20','?page=2&drive_page=2','?page=0&drive_page=-1','?page=999&drive_page=999','?type=documents&filename=alpha&drive_page=bogus']
files=paths.map do |query|
 path="/rooms/#{room.id}/files#{query}"
 browser.get path,headers:headers.dup
 body=browser.response.body
 start=body.index('<section class="room-files"')
 raise "Files #{path}: #{browser.response.status} #{body[0,400]}" unless start
 {path:,status:browser.response.status,content:body[start..body.index('</section>',start)+9]}
end
quotes=[[user,quote_room],[kevin,quote_room],[user,source_room],[kevin,source_room]].map do |viewer,r|
 browser.get "/rooms/#{r.id}/message_links/#{ref.id}",headers:headers_for.call(viewer)
 {viewer_id:viewer.id,room_id:r.id,status:browser.response.status,body:browser.response.body}
end
source_room.update_columns(deleted_at:Time.current)
browser.get "/rooms/#{quote_room.id}/message_links/#{ref.id}",headers:headers_for.call(user)
quotes<<{viewer_id:user.id,room_id:quote_room.id,status:browser.response.status,body:browser.response.body,deleted:true}
source_room.update_columns(deleted_at:nil)
ids=(messages+[source,quote]).map(&:id).join(',')
selects={'rooms'=>"id=#{room.id}",'memberships'=>"room_id=#{room.id}",'channel_threads'=>"id=#{thread.id}",'messages'=>"id IN (#{ids})",'action_text_rich_texts'=>"record_type='Message' AND record_id IN (#{ids})",'message_references'=>"message_id=#{quote.id}",'active_storage_blobs'=>"id IN (#{blobs.map(&:id).join(',')})",'active_storage_attachments'=>"id IN (#{uploads.map(&:id).join(',')})",'drive_attachments'=>"id IN (#{drives.map(&:id).join(',')})"}
rows=selects.to_h{|table,where|[table,ActiveRecord::Base.connection.select_all("SELECT * FROM #{table} WHERE #{where} ORDER BY id").to_a]}
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],rows:,room_id:room.id,source_id:source.id,quote_id:quote.id,reference_id:ref.id,files:,quotes:,card:,private_html:,sizes:) + "\n")
puts "WS8bm2 links/files Rails oracle: #{files.size} Files sections; #{quotes.size} quote HTTP responses; 2 quote partials; #{sizes.size} size values; #{rows.size} fixture tables"
