# Original WS12 recorder clauses: real source writers, subsequent preference changes,
# and complete persisted recipients. Every action commits, including its callbacks.
require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
extend ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActiveJob::Base.queue_adapter=:test
Rails.logger=ActiveSupport::Logger.new(File::NULL)
conn=ActiveRecord::Base.connection
id=->(name){ActiveRecord::FixtureSet.identify(name)}
room=id.call('designers'); author=id.call('jz'); recipient=id.call('david')
thread=901860001
common=["DELETE FROM sqlite_sequence WHERE name='messages'", "INSERT INTO sqlite_sequence(name,seq) VALUES('messages',9018650000)", "DELETE FROM activity_items", "DELETE FROM keyword_alerts", "UPDATE memberships SET involvement='mentions' WHERE room_id=#{room}",
 "INSERT INTO channel_threads(id,name,room_id,creator_id,created_at,updated_at,last_activity_at) VALUES(#{thread},'Recorder clauses',#{room},#{author},'2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')",
 "INSERT INTO thread_memberships(thread_id,user_id,involvement,joined_at,created_at,updated_at) VALUES(#{thread},#{author},'everything','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')",
 "INSERT INTO thread_memberships(thread_id,user_id,involvement,joined_at,created_at,updated_at) VALUES(#{thread},#{recipient},'everything','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')"]
msg=->(text, extra={}){{kind:'message',text:,**extra}}
member=->(involvement, thread=false){{kind:'involvement',involvement:,thread:}}
keyword=->(who=recipient){{kind:'keyword',user:who}}
cases={
 'c191'=>[keyword.call,msg.call('Redeploying the service')],
 'c192'=>[keyword.call(author),msg.call('Deploy now')],
 'c196'=>[keyword.call,member.call('nothing',true),msg.call('Deploy now',thread:true)],
 'c201'=>[msg.call('Hey @[David]')],
 'c203'=>[msg.call('Original',creator:recipient),msg.call('Hey @[David]',reply:true)],
 'c204'=>[member.call('mentions',true),msg.call('Quiet update',thread:true),member.call('everything',true),msg.call('Followed update',thread:true)],
 'c205'=>[{kind:'work',status:'planned'}],
 'c208'=>[member.call('nothing'),msg.call('Quiet',thread:true),member.call('mentions'),msg.call('Loud',thread:true)],
 'c210'=>[msg.call('Hey @[David]'),member.call('nothing'),member.call('invisible')],
 'c213'=>[msg.call('Original',thread:true,creator:recipient),msg.call('Hey @[David]',thread:true,reply:true)]
}
tables=%w[users rooms memberships thread_memberships channel_threads messages action_text_rich_texts work_thread_events activity_items keyword_alerts agent_events audit_logs sqlite_sequence]
saved=tables.to_h{|table|[table,conn.select_all("SELECT * FROM #{table}").to_a]}
restore=-> do
 conn.execute('PRAGMA foreign_keys=OFF')
 conn.execute('DELETE FROM message_search_index')
 saved.each{|table,rows|conn.execute("DELETE FROM #{table}");rows.each{|row|conn.execute("INSERT INTO #{table}(#{row.keys.join(',')}) VALUES(#{row.values.map{|v|conn.quote(v)}.join(',')})")}}
 conn.execute('PRAGMA foreign_keys=ON');ActiveSupport::IsolatedExecutionState.clear
end
snapshot=-> do
 ActivityItem.order(:user_id,:source_type,:source_id).map{|i|[i.user_id,i.source_type,i.source_id,i.event_type,i.read_at&.to_i,i.handled_at&.to_i]}
end
rows=cases.map do |key,actions|
 restore.call;common.each{|sql|conn.execute(sql)}
 last=nil
 steps=actions.map do |action|
  case action[:kind]
  when 'keyword' then KeywordAlert.create!(user_id:action[:user],phrase:'deploy')
  when 'involvement'
   model=action[:thread] ? ThreadMembership.find_by!(thread_id:thread,user_id:recipient) : Membership.find_by!(room_id:room,user_id:recipient)
   model.update!(involvement:action[:involvement])
  when 'message'
   last=Message.create!(room_id:room,creator_id:action[:creator]||author,thread_id:action[:thread] ? thread : nil,
     markdown_source:action[:text],reply_to_message_id:action[:reply] ? last.id : nil,client_message_id:"#{key}-#{SecureRandom.hex(3)}")
  when 'work' then ChannelThread.find(thread).update_work!(actor:User.find(author),work_status:action[:status])
  end
  {action:,items:snapshot.call}
 end
 {id:key,setup:common,steps:}
end
extra={}
restore.call
presence=%w[Thinking…].map{|text|Agents::WorkingPresence.set(agent:Agent.find(id.call('bender_agent')),text:).then{|r|{ok:r.ok?,payload:r.payload}}}
presence << Agents::WorkingPresence.set(agent:Agent.find(id.call('bender_agent')),text:'').then{|r|{ok:r.ok?,payload:r.payload}}
extra[:c148]=presence
restore.call;common.each{|sql|conn.execute(sql)}
agent=Agent.find(id.call('bender_agent'))
board_sql=["INSERT INTO channel_threads(id,name,room_id,creator_id,work_status,created_at,updated_at,last_activity_at) VALUES(901860002,'Unassigned post',#{room},#{author},'planned','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')", "UPDATE rooms SET type='Rooms::Board' WHERE id=#{room}","DELETE FROM agent_grants WHERE agent_id=#{agent.id}",
 "INSERT OR IGNORE INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(#{room},#{agent.user_id},'mentions','2026-03-02 16:00:00','2026-03-02 16:00:00')",
 "INSERT INTO agent_grants(agent_id,room_id,granted_by_id,capability,created_at,updated_at) VALUES(#{agent.id},#{room},#{recipient},'post_messages','2026-03-02 16:00:00','2026-03-02 16:00:00')"]
board_sql.each{|sql|conn.execute(sql)}
post=ChannelThread.find(thread);post.update_work!(actor:User.find(author),work_status:'planned',work_owner_id:agent.user_id)
humans,agents=post.work_owner_candidates
extra[:c177]={setup:common+board_sql,ids:ChannelThread.board_posts_for(post.room,owner:'agents').pluck(:id),human_kevin:humans.map(&:id).include?(id.call('kevin')),agent_bender:agents.map(&:id).include?(agent.user_id)}
restore.call
hroom=Room.find(id.call('david_and_jason'));caller=User.find(id.call('jason'));user=User.find(recipient)
grant=HuddleGrant.create!(id:901862001,room:hroom,user:caller,session:caller.sessions.first!,membership:hroom.memberships.find_by!(user:caller),identity:'ws12-named-missed',room_name:'ws12-named-missed',last_issued_at:Time.current)
item=ActivityItem.create!(id:901862002,user:,source:grant,event_type:'huddle_started')
frames=[]; sub=ActiveSupport::Notifications.subscribe('broadcast.action_cable'){|*args|e=args.last;frames << {stream:e[:broadcasting],payload:e[:message]} if e[:broadcasting]==ActivityChannel.stream_name_for(user.id)}
item.update!(event_type:'huddle_missed');ActiveSupport::Notifications.unsubscribe(sub)
columns=conn.columns('huddle_grants').map(&:name)
grant_setup="INSERT INTO huddle_grants(#{columns.join(',')}) VALUES(#{columns.map{|c|conn.quote(grant.attributes[c])}.join(',')})"
extra[:c143]={setup:["DELETE FROM activity_items",grant_setup,"INSERT INTO activity_items(id,user_id,source_type,source_id,event_type,created_at,updated_at) VALUES(#{item.id},#{recipient},'HuddleGrant',#{grant.id},'huddle_started','2026-03-02 16:00:00','2026-03-02 16:00:00')"],frames:}
extra[:c199]=[10,100].map do |size|
 restore.call;common.each{|sql|conn.execute(sql)}
 size.times do |i|
  uid=901863000+i
  conn.execute("INSERT INTO users(id,name,role,status,created_at,updated_at) VALUES(#{uid},'Quiet recipient',0,0,'2026-03-02 16:00:00','2026-03-02 16:00:00')")
  conn.execute("INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(#{room},#{uid},'mentions','2026-03-02 16:00:00','2026-03-02 16:00:00')")
 end
 [recipient,id.call('kevin')].each{|uid|KeywordAlert.create!(user_id:uid,phrase:'deploy')}
 message=Message.create!(room_id:room,creator_id:author,markdown_source:'Deploy now',client_message_id:"named-ceiling-#{size}")
 ActivityItem.delete_all
 queries=[];sub=ActiveSupport::Notifications.subscribe('sql.active_record'){|*args|e=args.last;queries << e[:sql] if !e[:cached] && e[:sql].match?(/\ASELECT/i)}
 ActivityItems::Recorder.record_message!(message)
 ActiveSupport::Notifications.unsubscribe(sub)
 {size:,items:snapshot.call,selects:queries.size}
end
files=%w[test/services/activity_items/recorder_test.rb test/services/activity_items/recorder_keyword_test.rb app/services/activity_items/recorder.rb app/models/notifications/policy.rb app/models/channel_thread.rb]
puts JSON.pretty_generate(sources:files.to_h{|f|[f,Digest::SHA256.file(Rails.root.join(f)).hexdigest]},rows:,extra:)
warn "WS12_RECORDER_REMAINING_RAILS #{rows.size+extra.size} named comparisons; #{rows.sum{|r|r[:steps].size}} committed writer/preference steps; complete recorded recipients; 0 masks"
