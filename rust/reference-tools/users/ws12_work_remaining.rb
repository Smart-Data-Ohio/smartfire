# Each previously flagged board/link clause runs a real request sequence and
# captures the complete response and committed facts. Request entropy is fixed.
require 'json'
require 'digest'
require 'action_dispatch/testing/integration'
require 'active_support/testing/time_helpers'
extend ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActiveJob::Base.queue_adapter=:test
Rails.logger=ActiveSupport::Logger.new(File::NULL)
ApplicationController.allow_forgery_protection=true
ApplicationController.skip_before_action :verify_authenticity_token,raise:false
[ChannelThreadsController,Threads::Work::LinksController].each{|c|c.skip_before_action :verify_authenticity_token,raise:false}
Rails.application.env_config['action_dispatch.show_exceptions']=:all
Rails.application.env_config['action_dispatch.content_security_policy_nonce_generator']=->(_){'NONCE'}
module Ws12WorkTokens
 def form_authenticity_token(form_options: {})
  action,method=form_options.values_at(:action,:method)
  action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
 end
end
ApplicationController.prepend(Ws12WorkTokens)
# Generated fixture identities are deterministic inputs, not response masks.
ws12_uuid=0
SecureRandom.define_singleton_method(:uuid){ws12_uuid+=1;format('00000000-0000-4000-8000-%012d',ws12_uuid)}
Random.define_singleton_method(:uuid){SecureRandom.uuid}
root=ENV.fetch('PARITY_WORK');conn=ActiveRecord::Base.connection
labels=JSON.parse(File.read(File.join(root,'parity/.seed/default/labels.json')))
tables=%w[users rooms memberships messages action_text_rich_texts channel_threads thread_memberships work_thread_events work_thread_links github_pull_requests events event_attendances event_references thread_tags activity_items agent_grants agents agent_credentials sqlite_sequence]
dump=-> do
 tables.flat_map{|t|columns=conn.columns(t).map(&:name);["DELETE FROM #{t}"]+conn.select_all("SELECT * FROM #{t} ORDER BY #{t=='sqlite_sequence' ? 'name' : 'id'}").map{|r|"INSERT INTO #{t}(#{columns.join(',')}) VALUES(#{columns.map{|c|conn.quote(r[c])}.join(',')})"}}
end
snapshot=-> {tables.to_h{|t|[t,conn.select_all("SELECT * FROM #{t}").to_a]}}
original_rows=snapshot.call
saved=dump.call
original_bad=conn.select_all("PRAGMA foreign_key_check").to_a

restore=-> do
 conn.execute('PRAGMA foreign_keys=OFF');conn.execute('DELETE FROM message_search_index');saved.each{|q|conn.execute(q)};conn.execute('PRAGMA foreign_keys=ON');ActiveSupport::IsolatedExecutionState.clear
end
board=699448332;channel=486777696;david=127326141;kevin=712064548;jz=773523953;bender=394959859;agent=773018776
agent_token='bender-test-secret-1234'
board_setup=JSON.parse(File.read(File.join(root,'vectors/boards_write.json')))['rows'][0]['setup']+["DELETE FROM activity_items","DELETE FROM memberships WHERE room_id=699448332 AND user_id=149087659","DELETE FROM thread_memberships WHERE thread_id IN(SELECT id FROM channel_threads WHERE room_id=699448332 AND id<>4)","DELETE FROM thread_tags WHERE channel_thread_id IN(SELECT id FROM channel_threads WHERE room_id=699448332 AND id<>4)","DELETE FROM messages WHERE room_id=699448332","DELETE FROM channel_threads WHERE room_id=699448332 AND id<>4","UPDATE channel_threads SET creator_id=773523953 WHERE id=4"]
human_setup=JSON.parse(File.read(File.join(root,'vectors/human_work_http.json')))['rows'][0]['setup']
req=->(path,method='get',params={},**extra){{path:,method:,params:,**extra}}
cases=[]
add=->(id,setup,steps,**extra){cases<<{id:,setup:,steps:,**extra}}
add.call('c111',board_setup,[req.call("/rooms/#{board}/threads.json",'post',{thread:{name:'Quiet post'}},viewer:'jz')])
grant="INSERT INTO agent_grants(agent_id,room_id,granted_by_id,capability,created_at,updated_at) VALUES(#{agent},#{board},#{david},'%s','2026-03-02 16:00:00','2026-03-02 16:00:00')"
agent_setup=board_setup+["INSERT OR IGNORE INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(#{board},#{bender},'mentions','2026-03-02 16:00:00','2026-03-02 16:00:00')","DELETE FROM agent_grants WHERE agent_id=#{agent}",grant%'read_messages',grant%'post_messages',"UPDATE channel_threads SET work_owner_id=#{bender} WHERE id=4"]
add.call('c112',agent_setup+[grant%'manage_threads'],[req.call('/agents/work/4','patch',{work_status:'in_progress',work_owner_id:kevin},agent:true)])
add.call('c113',agent_setup,[req.call('/agents/work/4','patch',{work_status:'in_progress'},agent:true)])
add.call('c114',board_setup,[req.call("/rooms/#{board}/threads/4/messages.json",'post',{message:{markdown_source:'A member reply',client_message_id:'board-reply'}},viewer:'kevin'),req.call("/rooms/#{board}/threads/4/join",'post',{},viewer:'kevin',accept:'text/html'),req.call("/rooms/#{board}/threads/4/leave",'delete',{},viewer:'kevin',accept:'text/html'),req.call("/rooms/#{board}/threads/4/messages.json",'post',{message:{markdown_source:'An intruder reply',client_message_id:'board-intruder'}},viewer:'jason')])
# Stable explicit IDs make the subsequent DELETE target the actual created link.
add.call('c115',board_setup+["DELETE FROM work_thread_links","DELETE FROM sqlite_sequence WHERE name='work_thread_links'"],[req.call('/threads/4/work/links','post',{kind:'drive_file',drive_url:'https://drive.google.com/file/d/board1234567'},viewer:'kevin',accept:'text/html'),req.call("/rooms/#{board}/threads/4",'get',{},viewer:'kevin',accept:'text/html'),req.call('/threads/4/work/links/1','delete',{},viewer:'kevin',accept:'text/html'),req.call('/threads/4/work/links','post',{kind:'drive_file',drive_url:'https://drive.google.com/file/d/board4567890'},viewer:'jason',accept:'text/html')])
[10,100].each do |size|
 setup=board_setup+["DELETE FROM thread_memberships WHERE thread_id=4","DELETE FROM channel_threads WHERE id=4"]+(0...size).map{|i|"INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,created_at,updated_at,last_activity_at) VALUES(#{9018950000+i},#{board},#{jz},'Query post #{i}','planned','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')"}
 add.call("c119-#{size}",setup,[req.call("/rooms/#{board}",'get',{},viewer:'jz',accept:'text/html')])
end
add.call('c120',board_setup+["UPDATE channel_threads SET work_status='in_progress' WHERE id=4","INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,created_at,updated_at,last_activity_at) VALUES(9018900001,#{board},#{jz},'Finished','done','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')"],[req.call("/rooms/#{board}?view=board",'get',{},viewer:'jz',accept:'text/html'),req.call("/rooms/#{board}?view=board&status=done",'get',{},viewer:'jz',accept:'text/html')])
add.call('c121',board_setup,[req.call("/rooms/#{board}/threads/4.json",'patch',{thread:{work_status:'in_progress'}},viewer:'jz')],broadcast_room:board)
add.call('c125',human_setup,[req.call('/rooms/486777696/threads/94.json','delete')],broadcast_room:channel)
restore.call
bot="#{bender}-BenderToken1"
add.call('c126',board_setup+["INSERT OR IGNORE INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(#{board},#{bender},'mentions','2026-03-02 16:00:00','2026-03-02 16:00:00')"],[req.call("/rooms/#{board}/#{bot}/messages",'post',{},raw:'Board root message',bot:true),req.call("/rooms/#{channel}/#{bot}/messages",'post',{},raw:'Channel root message',bot:true)])
# Original link fixture: three actual associations plus an unlinked upcoming event.
restore.call
human_setup.each{|q|conn.execute(q)}
t=ChannelThread.find(91);t.update_columns(name:'Linked work',work_status:'planned',work_owner_id:david)
WorkThreadLink.delete_all
pr=Github::PullRequest.for_reference(owner:'rails',repo:'rails',number:12);pr.update!(title:'Fix login',state:'open',html_url:'https://github.com/rails/rails/pull/12',private:false)
t.work_thread_links.create!(id:9018900011,kind: :pull_request,github_pull_request:pr,created_by:User.find(david))
t.work_thread_links.create!(id:9018900012,kind: :event,event:Event.find(ActiveRecord::FixtureSet.identify('watercooler_sync')),created_by:User.find(david))
t.work_thread_links.create!(id:9018900013,kind: :drive_file,url:'https://drive.google.com/file/d/1AbcDefGhIjKlMnOpQrSt/view',title:'Q3 Planning',created_by:User.find(david))
Event.create!(id:9018900014,room:Room.find(channel),organizer:User.find(david),title:'Next sync',starts_at:4.days.from_now,time_zone:'UTC')
link_setup=dump.call
add.call('c134',link_setup+["UPDATE github_pull_requests SET private=1 WHERE id=#{pr.id}"],[req.call("/rooms/#{channel}/threads/91",'get',{},accept:'text/html'),req.call("/rooms/#{channel}/threads/91",'get',{},accept:'text/html',sql:["UPDATE github_pull_requests SET private=NULL WHERE id=#{pr.id}"])])
add.call('c135',link_setup,[req.call("/rooms/#{channel}/threads/91",'get',{},accept:'text/html')])
add.call('c136',link_setup,[req.call('/work','get',{},accept:'text/html')])
add.call('c138',link_setup,[req.call("/rooms/#{channel}/threads/91",'get',{},accept:'text/html')])
add.call('c140',link_setup+["DELETE FROM sqlite_sequence WHERE name='work_thread_links'","INSERT INTO sqlite_sequence(name,seq) VALUES('work_thread_links',9018900013)"],[req.call('/threads/91/work/links','post',{kind:'event',event_id:9018900014},accept:'text/vnd.turbo-stream.html'),req.call('/threads/91/work/links/9018900014','delete',{},accept:'text/vnd.turbo-stream.html')])
add.call('c230',human_setup,[req.call('/rooms/486777696/threads/94.json')])
# Full eligible owner payload is captured by the ordinary-pane source; the profile
# fields and rejected owner variants use WS12's already pinned eligibility corpus.
restore.call
human_setup.each{|q|conn.execute(q)}
ChannelThread.find(91).update_columns(creator_id:jz,work_status:'planned',work_owner_id:nil)
Room.find(channel).memberships.grant_to(User.find(jz));Room.find(channel).memberships.grant_to(User.find(kevin))
%w[Eligible Suspended Outside Reader Botless].each do |kind|
 bot=User.create_bot!(name:"#{kind} Owner #{kind=='Botless' ? 'Bot' : 'Agent'}")
 bot.update_columns(bot_token_digest:Digest::SHA256.hexdigest("ws12-#{kind}"))
 Room.find(channel).memberships.grant_to(bot) unless kind=='Outside'
 next if kind=='Botless'
 agent_record=bot.create_agent!(kind: :workspace,owner:User.find(david))
 agent_record.update!(provider:'TestLab',description:'Does the work') if kind=='Eligible'
 agent_record.suspend! if kind=='Suspended'
 AgentGrant.create!(agent:agent_record,room:kind=='Outside' ? nil : Room.find(channel),granted_by:User.find(david),capability:kind=='Reader' ? 'read_messages' : 'post_messages')
end
add.call('c229',dump.call,[req.call('/rooms/486777696/threads/91.json','get',{},viewer:'jz')])
frames=[]
pubsub=ActionCable.server.pubsub
original=pubsub.method(:broadcast)
pubsub.define_singleton_method(:broadcast){|stream,message|frames<<{stream:,message:};original.call(stream,message)}
facts=-> do
 {threads:ChannelThread.order(:id).map{|t|[t.id,t.name,t.work_status,t.work_owner_id]},items:ActivityItem.order(:id).map{|i|[i.user_id,i.source_type,i.source_id,i.event_type,i.read_at&.iso8601(3),i.handled_at&.iso8601(3)]},links:WorkThreadLink.order(:id).map{|l|[l.id,l.channel_thread_id,l.kind,l.url,l.title,l.event_id,l.github_pull_request_id]},memberships:Membership.where(room_id:board).order(:user_id).map{|m|[m.user_id,m.unread?]},followers:ThreadMembership.order(:thread_id,:user_id).map{|m|[m.thread_id,m.user_id,m.involvement]},messages:Message.order(:id).map{|m|[m.id,m.creator_id,m.thread_id,m.markdown_source]}}
end
rows=cases.map do |entry|
 restore.call;conn.execute('PRAGMA foreign_keys=OFF');entry[:setup].each{|q|conn.execute(q)};conn.execute('PRAGMA foreign_keys=ON')
 bad=conn.select_all("PRAGMA foreign_key_check").to_a;raise "#{entry[:id]} fixture FK differences #{(bad-original_bad).inspect}" unless (bad-original_bad).empty?
 initial_setup=tables.flat_map do |table|
   key=table=='sqlite_sequence' ? 'name' : 'id'
   before=original_rows[table].index_by{|r|r[key]};after=snapshot.call[table].index_by{|r|r[key]}
   (before.keys-after.keys).map{|id|"DELETE FROM #{table} WHERE #{key}=#{conn.quote(id)}"} + after.filter_map do |id,row|
    if !before.key?(id)
     "INSERT INTO #{table}(#{row.keys.join(',')}) VALUES(#{row.values.map{|v|conn.quote(v)}.join(',')})"
    elsif before[id]!=row
     changes=row.reject{|col,value|before[id][col]==value}
     "UPDATE #{table} SET #{changes.map{|col,value|"#{col}=#{conn.quote(value)}"}.join(',')} WHERE #{key}=#{conn.quote(id)}"
    end
   end
  end
 browsers={}
 steps=entry[:steps].map do |step|
  Array(step[:sql]).each{|q|conn.execute(q)};frames.clear
  viewer=step[:viewer]||'david';browser=browsers[viewer]||=ActionDispatch::Integration::Session.new(Rails.application).tap{|b|b.host! 'campfire.test';b.cookies['session_token']=Rack::Utils.unescape(labels.fetch("session_cookies.#{viewer}"))}
  headers={'Accept'=>step[:accept]||'application/json','User-Agent'=>'Mozilla','Content-Type'=>step[:raw] ? 'text/plain' : 'application/json'}
  if step[:agent];browser.cookies.delete('session_token');headers['Authorization']=['Bearer',agent_token].join(' ');end
  step[:method]=='get' ? browser.get(step[:path],headers:) : browser.public_send(step[:method],step[:path],params:step[:raw]||step[:params].to_json,headers:)
  expected={ 'c111'=>[201], 'c112'=>[200], 'c113'=>[403], 'c114'=>[201,302,302,404], 'c115'=>[302,200,302,404], 'c121'=>[200], 'c125'=>[204], 'c126'=>[422,201], 'c229'=>[200] }
  if expected[entry[:id]];raise "#{entry[:id]} incorrect fixture status #{browser.response.status}" unless expected[entry[:id]][entry[:steps].index(step)]==browser.response.status;end
  if entry[:id]=='c229'
   options=JSON.parse(browser.response.body).dig('thread','work_owner_options');raise 'missing human Kevin' unless options.any?{|o|o['name']=='Kevin'}
   entry_agent=options.find{|o|o['name']=='Eligible Owner Agent'};raise 'missing profile' unless entry_agent && entry_agent['provider']=='TestLab' && entry_agent['description']=='Does the work' && entry_agent['agent']==true
   raise 'ineligible option' if options.any?{|o|o['name'].match?(/(Suspended|Outside|Reader|Botless) Owner/)}
  end
  selected=entry[:broadcast_room] ? frames.select{|f|f[:stream].end_with?(":#{entry[:broadcast_room]}:messages")} : []
  # Turbo's room stream uses a GID prefix, not a numeric namespace.
  selected=frames.select{|f|f[:stream]=="#{Room.find(entry[:broadcast_room]).to_gid_param}:messages"} if entry[:broadcast_room]
  selected=selected.map{|f|{stream:f[:stream],message:JSON.parse(f[:message])}}
  {**step,status:browser.response.status,body:browser.response.body,headers:%w[content-type cache-control pragma location].to_h{|k|[k,browser.response.headers[k]]},facts:facts.call,frames:selected}
 end
 {id:entry[:id],setup:initial_setup,steps:,broadcast_room:entry[:broadcast_room]}
end
sources=%w[test/controllers/channel_threads_board_test.rb test/integration/work_thread_links_test.rb test/controllers/channel_threads_controller_test.rb].to_h{|p|[p,Digest::SHA256.file(Rails.root.join(p)).hexdigest]}
puts JSON.pretty_generate(sources:,rows:)
warn "WS12_WORK_REMAINING_RAILS #{rows.size} named sequences; #{rows.sum{|r|r[:steps].size}} complete responses and committed facts; 0 masks"
