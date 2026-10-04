require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActionController::Base.allow_forgery_protection = false
user = User.find_by!(email_address: 'david@37signals.com')
room = user.rooms.find_by!(name: 'All Talk')
Current.user = user
queries = [ '', "\u{1c89}",  '  from:@jz, has:file launch  ', 'from:@jz from:@jz launch', 'has:PIN has:pin', 'is:THREAD', 'before:2026-03-03 after:2026-03-01 on:2026-03-02', 'on:2026-13-45 hello', 'from:@ in:# has:video is:board hello', 'xfrom:@jz in:#%', 'cats AND', 'foo NOT bar', '"quoted" OR NEAR(x*)', "a＿b café 日本 🙂", "from:@jz\u00a0has:pin" ]
parsed = queries.map do |raw|
 q = SearchQuery.parse(raw)
 { raw:, text: q.text, expression: q.match_expression, blank: q.blank_query?, filters: q.filters?, chips: q.chips.map(&:to_h) }
end
renderer = ApplicationController.renderer.new(http_host: 'campfire.test', https: false)
partials = parsed.map do |entry|
 {raw:entry[:raw], html:renderer.render(partial:'searches/filters',assigns:{search_query:SearchQuery.parse(entry[:raw])})}
end
empty = renderer.render(template:'searches/index',layout:false,assigns:{query:nil,search_query:SearchQuery.parse(''),recent_searches:[],return_to_room:nil,messages:[],board_posts:[],work_threads:[],events:[],has_more_older:false})
clear = renderer.render(template:'searches/clear',formats:[:turbo_stream],layout:false)
dates = []
[['UTC','2026-03-02'],['Hawaii','2026-03-02'],['Eastern Time (US & Canada)','2026-03-08'],['Eastern Time (US & Canada)','2026-11-01'],['America/Sao_Paulo','2018-11-04'],['Pacific/Apia','2011-12-30']].each_with_index do |(zone,day),case_id|
 Time.use_zone(zone) do
  date = Date.iso8601(day)
  first = date.in_time_zone.beginning_of_day
  last = date.in_time_zone.end_of_day
  times = [first-0.000001,first,first+0.000001,first+12.hours,last,last+0.000001,first-1.day,first+1.day].map { |time| time.utc.round(6) }
  tag="dateprobe#{case_id}"
  records = times.each_with_index.map { |time,i| room.messages.create!(creator:user,body:"#{tag} sample#{i}",created_at:time,client_message_id:"#{tag}-#{i}") }
  results = %w[on before after].to_h do |op|
   selected = SearchQuery.parse("#{op}:#{day}").apply_to_messages(Message.where(id: records.map(&:id))).pluck(:id)
   [op, records.each_index.select{|i|selected.include?(records[i].id)}]
  end
  dates << {zone:,day:,tag:,times:times.map{|time|time.iso8601(6)},results:}
 end
end
user.update_columns(time_zone: 'UTC')
room.channel_threads.create!(creator:user,name:'Oracle launch work',work_status:'planned')
board = Rooms::Board.create!(creator:user,name:'Oracle Board')
board.memberships.grant_to([user])
board.channel_threads.create!(creator:user,name:'Oracle launch plan',work_status:'planned')
Event.create!(room:,organizer:user,title:'Oracle launch gathering',starts_at:Time.utc(2026,3,3,17),time_zone:'UTC')
sections_query = SearchQuery.parse('oracle launch')
sections = renderer.render(partial:'searches/sections', assigns:{ board_posts:sections_query.board_posts_for(user).to_a,work_threads:sections_query.work_threads_for(user).to_a,events:sections_query.events_for(user).to_a })
sections_data = [['board-posts', sections_query.board_posts_for(user).to_a], ['work-threads', sections_query.work_threads_for(user).to_a], ['events', sections_query.events_for(user).to_a]].map do |kind,records|
 {kind:,records:records.map{|r|{id:r.id,room_id:r.room_id,room_type:r.room.type,room_name:r.room.name,title:r.is_a?(Event) ? r.title : r.name,time:(r.is_a?(Event) ? r.starts_at : r.last_activity_at).utc.iso8601(6),status:r.is_a?(Event) ? nil : r.work_status,cancelled:r.is_a?(Event) && r.cancelled?}}}
end
older = renderer.render(partial:'searches/load_older',locals:{query:'from:@jz has:file launch',oldest_id:123})
older_empty = renderer.render(template:'searches/index',formats:[:turbo_stream],layout:false,assigns:{messages:[],has_more_older:false})
request = ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.input'=>StringIO.new))
request.cookie_jar.signed[:session_token] = user.sessions.where.not(two_factor_verified_at:nil).first!.token
headers = {'Cookie'=>"session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"}
browser=ActionDispatch::Integration::Session.new(Rails.application)
browser.host! 'campfire.test'
steps = []
[[:post,'/searches',{q:'  from:@jz  has:file launch  '},'text/html'],[:post,'/searches',{q:'???'},'text/html'],[:delete,'/searches/clear',{},'application/json'],[:delete,'/searches/clear',{},'text/vnd.turbo-stream.html']].each do |method,path,input,accept|
 browser.public_send(method,path,params:input,headers:headers.merge('Accept'=>accept))
 steps << {method:,path:,input:,accept:,status:browser.response.status,location:browser.response.headers['Location'],body:browser.response.body,content_type:browser.response.media_type}
end
word_ranges = []
start = nil
(0..0x10ffff).each do |point|
  word = !(0xd800..0xdfff).cover?(point) && point.chr(Encoding::UTF_8).match?(/\A[[:word:]]\z/)
  if word
    start ||= point
  elsif start
    word_ranges << [start, point - 1]
    start = nil
  end
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],parsed:,partials:,empty:,clear:,dates:,sections:,sections_data:,older:,older_empty:,steps:,word_ranges:)+"\n")
puts "WS8bm2 search Rails oracle: #{parsed.size} parsed queries; #{partials.size} chip partials; 1 empty page; 1 clear stream; #{dates.size*3} zone/date selections; 1 populated sections partial; 1 load-older control; 1 empty older stream; #{steps.size} HTTP responses; #{word_ranges.size} Unicode word ranges"
