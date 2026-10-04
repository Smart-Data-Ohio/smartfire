# Exact Time.zone.parse / slash fallback exceptions and real structured request params.
require 'json'
require 'action_dispatch/testing/integration'
require_relative 'oracle-database'
ActiveJob::Base.queue_adapter=:test
ActionController::Base.allow_forgery_protection=false
Rails.application.config.hosts.clear
inputs=["5th March 2026", "Thu, 05 Mar 2026 14:30:00 GMT", "Thu Mar 5 14:30:00 2026", "H38.03.05", "R08.03.05", "2026-064", "2026-W10-4", "--0305", "---05", "2026.064", "2026.03.05", "2026/3/5", "March 5 AD 2026", "5 Mar 26 BC", "'26 Mar 5", "20260305T143000.123456789+0530", "20260305 14:30 +05:30", "20260305143000[+05:30]", "2026-03-05 14:30:00 +5.5", "2026-03-05 14:30:00 UTC+5", "2026-03-05 14:30:00 Eastern Standard Time", "2026-03-05 14:30:00 +2500", "2026-03-05 14:30:00.9999999999Z", "2026-03-05 12:30:60Z", "2026-03-05T24:00:00.5Z", "2026-02-31", "2026-13-05", "2026-03-32", "14:60", "25:00", "2026-03-05 2147483648:00", "999999999999999999999999-03-05", "-0001-03-05", "1.30pm", "14h30m15s", "123", "15", "0305", "202603", "2026", "2026-03", "Mar", "junk", "line\nMarch 5 2026", "\u00a0March 5 2026\u00a0"]
zones=%w[UTC America/New_York America/Chicago Australia/Lord_Howe Pacific/Apia]
dst=['2026-03-08 02:00','2026-03-08 02:30','2026-11-01 01:00','2026-11-01 01:30','2026-10-04 02:00','2026-10-04 02:15','2026-04-05 01:30','2026-04-05 01:45','2011-12-30 12:00']
def result
 t=yield
 {result:t&.utc&.strftime('%Y-%m-%d %H:%M:%S.%6N')}
rescue ArgumentError,TypeError,RangeError=>e
 {error:e.class.name,message:e.message}
end
cases=zones.flat_map { |zone| Time.use_zone(zone) { (inputs+dst+dst.map{|s|s+' -05:00'}).map { |input| {zone:,input:,calendar:result {Time.zone.parse(input)},slash:result {SlashCommands::TimeParser.parse(input,zone:)}} } } }
user=User.find(127326141);room=Room.find(699448326)
message=room.messages.create!(creator:user,markdown_source:'Exceptional input reference',client_message_id:'exceptional-input-reference')
rows={'messages'=>ActiveRecord::Base.connection.select_all("SELECT * FROM messages WHERE id=#{message.id}").to_a,'action_text_rich_texts'=>ActiveRecord::Base.connection.select_all("SELECT * FROM action_text_rich_texts WHERE record_type='Message' AND record_id=#{message.id}").to_a}
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
headers={'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
reset=MessagingOracleDatabase.scenarios(ARGV.fetch(0));steps=[]
values=['2026-03-05 2147483648:00','2026-03-05 14:60','2026-03-05 14:30 +2500','x'*129,[],['2026-03-05 14:30'],['2026-03-05 14:30','junk'],{}, {'at'=>'2026-03-05 14:30'},nil,false,true,17]
values.each do |value|
 [[:post,'/saved',{message_id:message.id,saved_item:{remind_at:value}}],[:post,"/rooms/#{room.id}/scheduled_messages",{scheduled_message:{send_at:value,markdown_source:'exceptional scheduled'}}],[:post,"/rooms/#{room.id}/slash_commands",{text:value}]].each do |method,path,params|
  reset.call do
   browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
   browser.public_send(method,path,params:,headers:headers.dup,as: :json)
   steps << {method:method.to_s.upcase,path:,params:,status:browser.response.status,content_type:browser.response.headers['Content-Type'],body:browser.response.body}
  end
 end
end
[[:post,'/saved',{message_id:message.id,saved_item:{'remind_at(1i)'=>'2026','remind_at(2i)'=>'03','remind_at(3i)'=>'05','remind_at(4i)'=>'14','remind_at(5i)'=>'30'}}],[:post,"/rooms/#{room.id}/scheduled_messages",{scheduled_message:{'send_at(1i)'=>'2026','send_at(2i)'=>'03','send_at(3i)'=>'05','send_at(4i)'=>'14','send_at(5i)'=>'30',markdown_source:'multiparameter scheduled'}}]].each do |method,path,params|
 reset.call do
  browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
  browser.public_send(method,path,params:,headers:headers.dup,as: :json)
  steps << {method:method.to_s.upcase,path:,params:,status:browser.response.status,content_type:browser.response.headers['Content-Type'],body:browser.response.body}
 end
end
["/searches?q[]=unmatched_exceptional", "/searches?q[x]=unmatched_exceptional", "/searches?q=hello&before[]=2", "/searches?q=hello&before[x]=2", "/rooms/#{room.id}/files?filename[]=hello", "/rooms/#{room.id}/files?filename[x]=hello", "/rooms/#{room.id}/files?page[]=2&type[]=images", "/rooms/#{room.id}/files?drive_page[x]=2&type[x]=images", "/autocompletable/users.json?page[x]=2"].each do |path|
 reset.call do
  browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test';accept=path.include?('/autocompletable/') ? 'application/json' : 'text/html';browser.get(path,headers:headers.merge('Accept'=>accept))
  body=browser.response.body;marker=body.include?('<section class="room-files"') ? '<section class="room-files"' : body.include?('<section id="message-area"') ? '<section id="message-area"' : nil;if marker
   start=body.index(marker);depth=0;finish=nil
   body[start..].to_enum(:scan,/<section\b[^>]*>|<\/section>/).each do
    m=Regexp.last_match;depth+=m[0].start_with?('</') ? -1 : 1
    if depth==0;finish=start+m.end(0);break;end
   end
   raise 'missing balanced owned section' unless finish
   body=body[start...finish]
  end;steps << {method:'GET',path:,accept:,marker:,status:browser.response.status,content_type:browser.response.headers['Content-Type'],body:}
 end
end
['until 2026-03-05 2147483648:00','until 2026-03-05 14:60'].each do |input|
 reset.call do
  browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
  params={text:"/dnd #{input}"};path="/rooms/#{room.id}/slash_commands"
  browser.post(path,params:,headers:headers.dup,as: :json)
  steps << {method:'POST',path:,params:,status:browser.response.status,content_type:browser.response.headers['Content-Type'],body:browser.response.body}
 end
end
source=Room.find(654632876).messages.first!
ref=MessageReference.create!(message:,referenced_message:source)
rows['message_references']=ActiveRecord::Base.connection.select_all("SELECT * FROM message_references WHERE id=#{ref.id}").to_a
["/rooms/#{room.id}/message_links/#{ref.id}?id[]=#{ref.id}","/rooms/#{room.id}/message_links/#{ref.id}?id[x]=#{ref.id}","/rooms/#{room.id}/message_links/#{ref.id}?room_id[]=#{room.id}","/rooms/#{room.id}/message_links/#{ref.id}?room_id[x]=#{room.id}"].each do |path|
 browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test';browser.get(path,headers:headers.merge('Accept'=>'text/html'))
 steps << {method:'GET',path:,accept:'text/html',marker:nil,status:browser.response.status,content_type:browser.response.headers['Content-Type'],body:browser.response.body}
end
[['America/New_York',7],['America/Chicago',7],['Australia/Lord_Howe',35],['UTC',7]].each do |zone,days|
['dnd','ooo'].each do |command|
 input="/#{command} #{days}d"
 reset.call do
  user.reload.update_columns(time_zone:zone)
  browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
  params={text:input};path="/rooms/#{room.id}/slash_commands"
  browser.post(path,params:,headers:headers.dup,as: :json)
  state=user.reload.attributes.slice('dnd_until','ooo_until').transform_values{|time|time&.utc&.strftime('%Y-%m-%d %H:%M:%S.%6N')}
  steps << {method:'POST',path:,params:,status:browser.response.status,content_type:browser.response.headers['Content-Type'],body:browser.response.body,state:,zone:}
 end
end;end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],cases:,rows:,steps:)+"\n")
puts "WS8bm2 exceptional inputs Rails: #{cases.size} exact calendar/slash result pairs; #{steps.size} structured HTTP status/type/body comparisons"
