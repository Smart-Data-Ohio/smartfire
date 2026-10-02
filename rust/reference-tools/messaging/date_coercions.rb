require 'json'
inputs=['26-03-05','69-03-05','68-03-05','05/03/26','2026-03','2026','03/05','3/5','March 5','5 Mar 26','March 5 26','Mar','5th March','2026-064','2026-W10-4','2026.03.05','2026/3/5','202603','260305','0305','15','123','1:30pm','1.30pm','14h30','14:30 UTC+5','14:30 +05:30','2026-03-05 12:00:00,5','2026-03-05T12:00:00.000000999Z','2026-03-05T24:00:00.5Z','2026-03-05 12:30:60Z','2026-03-05 13:00 PST','Thu Mar 5 14:30:00 2026','2026-13-05','32/03/2026','March 32 2026','20261305','20260332']
shapes=[nil,false,true,20260305,17,1.5,[],['2026-03-05 14:30'],['2026-03-05 14:30','junk'],{}, {'at'=>'2026-03-05 14:30'},['x',{'at'=>'2026-03-05 14:30'}],"line\nquote\"\\",'  ','in_progress']
coercions=shapes.map do |input|
 param=ActionController::Parameters.new(value:input)[:value]
 {input:,string:param.to_s,present:param.present?}
end
cases=%w[UTC America/New_York].flat_map do |zone|
 Time.use_zone(zone) do
  (inputs+coercions.map{|c|c[:string]}).map do |input|
   begin
    time=Time.zone.parse(input)
    {zone:,input:,parts:Date._parse(input,false),result:time&.utc&.iso8601(6)}
   rescue ArgumentError,TypeError=>e
    {zone:,input:,parts:Date._parse(input,false),error:e.class.name}
   end
  end
 end
end
ActionController::Base.allow_forgery_protection=false
Rails.application.config.hosts.clear
user=User.find(127326141);room=Room.find(699448326)
message=room.messages.create!(creator:user,markdown_source:'Parameter calendar reference',client_message_id:'coercion-calendar-reference')
request=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token]=user.sessions.where.not(two_factor_verified_at:nil).first!.token
headers={'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
browser=ActionDispatch::Integration::Session.new(Rails.application);browser.host! 'campfire.test'
steps=[]
[shapes[7],shapes[10],false,nil,17,1.5,[],shapes[8]].each do |value|
 input={message_id:message.id,saved_item:{remind_at:value}}
 browser.post('/saved',params:input,headers:headers.dup,as: :json)
 steps<<{input:,status:browser.response.status,body:browser.response.body}
end
ids=message.id
rows={'messages'=>ActiveRecord::Base.connection.select_all("SELECT * FROM messages WHERE id=#{ids}").to_a,'action_text_rich_texts'=>ActiveRecord::Base.connection.select_all("SELECT * FROM action_text_rich_texts WHERE record_type='Message' AND record_id=#{ids}").to_a}
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',coercions:,cases:,rows:,steps:)+"\n")
puts "WS8bm2 broader date/coercion Rails oracle: #{cases.size} calendar cases; #{coercions.size} parameter string/presence probes; #{steps.size} reminder HTTP responses"
