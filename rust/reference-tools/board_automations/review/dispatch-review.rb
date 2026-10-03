module OwnDispatchProbe
  extend self
require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
ActiveJob::Base.queue_adapter = :test
Rails.logger = ActiveSupport::Logger.new(File::NULL)
CONN = ActiveRecord::Base.connection
NOW = Time.utc(2026,3,2,16)
travel_to NOW
JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'reference-tools/board_automations/dispatch-source-hashes.json'))).each do |file, hash|
  raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest == hash
end
TABLES = %w[rooms users memberships agents agent_grants channel_threads board_sla_rules board_sla_nudges board_stale_digests activity_items messages action_text_rich_texts message_references]
SNAPSHOT = TABLES.to_h { |t| [t,CONN.select_all("SELECT * FROM #{t}").to_a] }
SEQUENCES = CONN.select_all('SELECT * FROM sqlite_sequence').to_a
def restore_seed
  CONN.execute('PRAGMA foreign_keys=OFF')
  CONN.transaction do
    CONN.execute('DROP TRIGGER IF EXISTS own_reject_digest')
    TABLES.reverse_each { |t| CONN.execute("DELETE FROM #{t}") }
    SNAPSHOT.each do |table,rows|
      rows.each { |r| CONN.execute("INSERT INTO #{table}(#{r.keys.join(',')}) VALUES(#{r.values.map { |v| CONN.quote(v) }.join(',')})") }
    end
    CONN.execute('DELETE FROM sqlite_sequence')
    SEQUENCES.each { |r| CONN.execute("INSERT INTO sqlite_sequence(name,seq) VALUES(#{CONN.quote(r['name'])},#{r['seq']})") }
  end
  CONN.execute('PRAGMA foreign_keys=ON')
  ActiveRecord::Base.clear_query_caches_for_current_thread
  ActiveJob::Base.queue_adapter.enqueued_jobs.clear
end
def user(id,name,role=0,status=0)
  "INSERT INTO users(id,name,role,status,created_at,updated_at) VALUES(#{id},#{CONN.quote(name)},#{role},#{status},'2026-03-02 16:00:00.000000','2026-03-02 16:00:00.000000')"
end
def board(id,creator,members)
  ["INSERT INTO rooms(id,name,type,creator_id,created_at,updated_at) VALUES(#{id},'Own review board','Rooms::Board',#{creator},'2026-03-02 16:00:00.000000','2026-03-02 16:00:00.000000')",*members.map { |u| "INSERT INTO memberships(room_id,user_id,created_at,updated_at,unread_at) VALUES(#{id},#{u},'2026-03-02 16:00:00.000000','2026-03-02 16:00:00.000000',NULL)" }]
end
def rule(id,room,status,nudge=60,escalate=240)
  "INSERT INTO board_sla_rules(id,room_id,work_status,nudge_after_minutes,escalate_after_minutes,created_at,updated_at) VALUES(#{id},#{room},#{CONN.quote(status)},#{nudge},#{escalate},'2026-03-02 16:00:00.000000','2026-03-02 16:00:00.000000')"
end
def thread(id,room,status,entered,owner=nil,title='Own <&> **literal** post')
  "INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_owner_id,work_status_changed_at,created_at,updated_at,last_activity_at) VALUES(#{id},#{room},971100001,#{CONN.quote(title)},#{CONN.quote(status)},#{owner || 'NULL'},#{CONN.quote(entered)},'2026-03-02 16:00:00.000000','2026-03-02 16:00:00.000000','2026-03-02 16:00:00.000000')"
end
def common
  ['DELETE FROM board_sla_nudges','DELETE FROM board_sla_rules','DELETE FROM board_stale_digests','DELETE FROM activity_items',
   user(971100001,'Creator <&>'),user(971100002,'Other creator'),user(971100003,'Owner ** <&>'),user(971100004,'Inactive owner',0,1),user(971100005,'Removed owner'),user(971100006,'Active agent human'),user(971100007,'Inactive agent human',0,1),user(971100008,'Removed agent human'),
   user(971100009,'Bot creator',2),user(971100010,'Active bot',2),user(971100011,'Inactive suspended bot',2,1),user(971100012,'Inactive human bot',2),user(971100013,'Removed human bot',2),
   *[[10,6],[11,6],[12,7],[13,8]].map { |b,h| "INSERT INTO agents(id,user_id,owner_id,status,suspended_at,created_at,updated_at) VALUES(9712000#{b},9711000#{format('%02d',b)},9711000#{format('%02d',h)},'idle',#{b==11 ? "'2026-03-02 15:00:00'" : 'NULL'},'2026-03-02 16:00:00','2026-03-02 16:00:00')" }]
end
def sla_facts
  {claims:BoardSlaNudge.order(:id).pluck(:room_id,:channel_thread_id,:work_status,:stage,:recipient_id,:status_entered_at).map { |r|r[5]=r[5].strftime('%Y-%m-%d %H:%M:%S.%6N');r },
   items:ActivityItem.where(source_type:'BoardSlaNudge').order(:id).pluck(:user_id,:event_type,:source_type,:source_id,:read_at,:handled_at).map { |r|r[3]=BoardSlaNudge.find(r[3]).channel_thread_id;r },
   jobs:ActiveJob::Base.queue_adapter.enqueued_jobs.map { |j| n=GlobalID::Locator.locate(j[:args][0]['_aj_globalid']);[j[:job].name,n.recipient_id,n.channel_thread_id,n.stage] }}
end
def digest_facts(before,rooms)
  {claims:BoardStaleDigest.order(:id).map { |d| {room_id:d.room_id,on:d.digest_on.to_s,attached:!!d.message_id} },
   notes:Message.where(system_note:true,room_id:rooms).order(:id).map { |m|{room_id:m.room_id,creator_id:m.creator_id,body:m.body.body.to_html,plain:m.plain_text_body,system_note:m.system_note,thread_id:m.thread_id,streaming:m.streaming} },
   message_delta:Message.count-before,items:ActivityItem.count,unread:Membership.where(room_id:rooms).where.not(unread_at:nil).count,jobs:ActiveJob::Base.queue_adapter.enqueued_jobs.map { |j|j[:job].name }}
end
specs=[]
# Three board creators; mixed owners and distinct rules preserve sweep/recipient order.
sql=common
sql += board(971000001,971100001,[971100001,971100003,971100004,971100006,971100007])
sql += board(971000002,971100002,[971100002,971100003,971100006])
sql += board(971000003,971100009,[971100009,971100003])
[['planned',30,120],['in_progress',60,240],['blocked',1,2],['done',1,2]].each_with_index { |(s,n,e),i|sql << rule(971300001+i,971000001,s,n,e) }
sql << rule(971300010,971000002,'in_progress') << rule(971300011,971000003,'in_progress')
entries=[['planned','2026-03-02 15:30:00.000001',3],['planned','2026-03-02 15:30:00.000000',3],['planned','2026-03-02 14:00:00.000001',nil],['planned','2026-03-02 14:00:00.000000',1],['in_progress','2026-03-02 15:00:00.000001',3],['in_progress','2026-03-02 15:00:00.000000',4],['in_progress','2026-03-02 12:00:00.000001',5],['in_progress','2026-03-02 12:00:00.000000',10],['in_progress','2026-03-02 11:00:00.000000',11],['in_progress','2026-03-02 11:00:00.000000',12],['in_progress','2026-03-02 11:00:00.000000',13],['blocked','2026-03-02 15:58:00.000000',3],['done','2026-03-02 10:00:00.000000',3],['in_progress',nil,3]]
entries += [['planned','2026-03-02 15:29:59.999999',3],['planned','2026-03-02 13:59:59.999999',3],['in_progress','2026-03-02 14:59:59.999999',3],['in_progress','2026-03-02 11:59:59.999999',3]]
entries.each_with_index { |(s,at,o),i|sql << thread(971400001+i,971000001,s,at,o && 971100000+o) }
sql << thread(971400030,971000002,'in_progress','2026-03-02 11:00:00.000000',971100003)
sql << thread(971400031,971000003,'in_progress','2026-03-02 11:00:00.000000',nil)
sql << thread(971400032,971000003,'in_progress','2026-03-02 11:00:00.000000',971100003)
sql << "INSERT INTO agent_grants(agent_id,room_id,capability,granted_by_id,revoked_at,created_at,updated_at) VALUES(971200011,971000001,'post',971100001,'2026-03-02 15:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')"
specs << {name:'mixed-sla-crossings',kind:'sla',setup:sql,rooms:[971000001,971000002,971000003],runs:[{now:NOW,setup:[]},{now:NOW,setup:[]},{now:NOW+60,setup:["UPDATE channel_threads SET work_status_changed_at='2026-03-02 11:00:00.123456' WHERE id=971400008","UPDATE channel_threads SET work_status='blocked',work_status_changed_at='2026-03-02 15:58:00.000000' WHERE id=971400002"]}]}
# 25 due posts, >20 truncation, tie IDs, escaping, minute/hour/day age transitions.
dnow=Time.utc(2026,3,2,23,59,59,999999)
sql=common+board(971000001,971100001,[971100001,971100003])
sql += [rule(971300001,971000001,'planned',1,240),rule(971300002,971000001,'in_progress',60,240),rule(971300003,971000001,'blocked',60,240),rule(971300004,971000001,'done',1,2)]
25.times do |i|
  age=[1440,1439,61,60,59,1][i%6]
  s=age<60 ? 'planned' : %w[planned in_progress blocked][i%3]
  sql << thread(971400001+i,971000001,s,(dnow-age*60).strftime('%Y-%m-%d %H:%M:%S.%6N'),i%4==0 ? nil : 971100003,"#{i} **Title** <tag> & \"quoted\" 'apostrophe'")
end
sql << thread(971400040,971000001,'blocked',(dnow-3600+Rational(1,1_000_000)).strftime('%Y-%m-%d %H:%M:%S.%6N'),971100003,'One microsecond before threshold')
sql << thread(971400041,971000001,'done',(dnow-86400).strftime('%Y-%m-%d %H:%M:%S.%6N'),971100003,'Done excluded')
specs << {name:'mixed-digest-25-utc-rollover',kind:'digest',setup:sql,rooms:[971000001],runs:[{now:dnow,setup:[]},{now:dnow,setup:[]},{now:dnow+Rational(1,1_000_000),setup:[]}]}
%w[post attach].each do |failure|
  sql=common+board(971000001,971100001,[971100001,971100003])+board(971000002,971100002,[971100002,971100003])
  [1,2].each { |i|sql << rule(971300000+i,971000000+i,'planned',1,240) << thread(971400000+i,971000000+i,'planned','2026-03-02 14:00:00.000000',971100003) }
  trigger=failure=='post' ? "CREATE TRIGGER own_reject_digest BEFORE INSERT ON messages WHEN NEW.system_note=1 AND NEW.room_id=971000001 BEGIN SELECT RAISE(ABORT,'own reject post'); END" : "CREATE TRIGGER own_reject_digest BEFORE UPDATE OF message_id ON board_stale_digests WHEN NEW.room_id=971000001 BEGIN SELECT RAISE(ABORT,'own reject attach'); END"
  specs << {name:"committed-#{failure}-failure-healthy-board",kind:'digest',setup:sql+[trigger],rooms:[971000001,971000002],runs:[{now:NOW,setup:[]},{now:NOW,setup:['DROP TRIGGER own_reject_digest']}]}
end
%w[sla digest].each do |kind|
  [10,100].each do |size|
    sql=common
    size.times do |i|
      room=972000000+i
      creator=i.even? ? 971100001 : 971100002
      sql+=board(room,creator,[creator,971100003,971100006])
      sql << rule(972300000+i*2,room,'planned',30,120) << rule(972300001+i*2,room,'blocked',60,240)
      sql << thread(972400000+i*3,room,'planned','2026-03-02 13:00:00.000000',i%3==0 ? nil : 971100003)
      sql << thread(972400001+i*3,room,'blocked','2026-03-02 11:00:00.000000',971100011)
      sql << thread(972400002+i*3,room,'blocked','2026-03-02 15:00:00.000001',971100003)
    end
    specs << {name:"#{kind}-mixed-#{size}",kind:kind,setup:sql,rooms:(972000000...972000000+size).to_a,count_size:size,runs:[{now:NOW,setup:[]},{now:NOW,setup:[]}]}
  end
end
sources = SNAPSHOT['messages'].reject { |m| m['system_note'] == 1 }.first(64)
raise 'need 64 quoted source messages' unless sources.size == 64
sql = common
32.times do |i|
  room = 973000000 + i
  urls = sources.slice(i * 2, 2).map { |m| "/rooms/#{m['room_id']}/@#{m['id']}" }.join(' ')
  sql += board(room, 971100001, [971100001])
  sql << rule(973300000 + i, room, 'planned', 30, 120)
  sql << thread(973400000 + i, room, 'planned', '2026-03-02 13:00:00.000000', nil, urls)
end
specs << {name:'digest-quoted-titles-bind-limit',kind:'digest',setup:sql,rooms:(973000000...973000032).to_a,bind_limit:64,runs:[{now:NOW,setup:[]}]}
results=[]
specs.each do |spec|
  # Rails SQLite quotes whole-second Time without a zero fractional suffix.
  spec[:setup].map! { |sql| sql.gsub(/(?<=\d{2}:\d{2}:\d{2})\.000000/, '') }
  spec[:runs].each { |run| run[:setup].map! { |sql| sql.gsub(/(?<=\d{2}:\d{2}:\d{2})\.000000/, '') } }
  restore_seed
  CONN.transaction { spec[:setup].each { |s|raise("bad setup #{s.inspect}") unless s.is_a?(String); CONN.execute(s) } }
  before=Message.count
  spec[:runs].each do |run|
    CONN.transaction { run[:setup].each { |s|raise("bad setup #{s.inspect}") unless s.is_a?(String); CONN.execute(s) } }
    travel_to run[:now]
    selects=[]
    subscriber=ActiveSupport::Notifications.subscribe('sql.active_record') { |*a| e=a.last;selects << e[:sql] if !e[:cached] && e[:name]!='SCHEMA' && e[:sql].match?(/\A\s*SELECT/i) }
    begin
      spec[:kind]=='sla' ? BoardAutomations::SlaDispatcher.dispatch_due!(now:run[:now]) : BoardAutomations::DigestDispatcher.dispatch_due!(now:run[:now])
    ensure
      ActiveSupport::Notifications.unsubscribe(subscriber)
    end
    run[:reads]=selects.size
    if spec[:bind_limit]
      quoted = MessageReference.joins(:message).where(messages: {system_note: true,room_id: spec[:rooms]}).distinct.count(:referenced_message_id)
      raise "expected 64 quoted sources, got #{quoted}" unless quoted == 64
    end
    run[:facts]=spec[:kind]=='sla' ? sla_facts : digest_facts(before,spec[:rooms])
    run[:now]=run[:now].strftime('%Y-%m-%d %H:%M:%S.%6N')
  end
  warn JSON.generate(name:spec[:name],reads:spec[:runs].map { |r|r[:reads] },facts:spec[:runs].map { |r|r[:facts].slice(:message_delta,:unread).merge(items:r[:facts][:items].is_a?(Array) ? r[:facts][:items].size : r[:facts][:items],claims:r[:facts][:claims].size,jobs:r[:facts][:jobs].size) })
  results << spec
end
puts JSON.pretty_generate(now:NOW.strftime('%Y-%m-%d %H:%M:%S.%6N'),cases:results)

end
