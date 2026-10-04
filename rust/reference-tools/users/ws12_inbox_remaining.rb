# Complete named HTTP clauses left out of the broad inbox corpus. Fixed request
# entropy matches existing golden-token tests; bodies and saved facts are unmasked.
require 'json'
require 'digest'
require 'action_dispatch/testing/integration'
require 'active_support/testing/time_helpers'
extend ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActiveJob::Base.queue_adapter=:test
Rails.logger=ActiveSupport::Logger.new(File::NULL)
ApplicationController.allow_forgery_protection=true
ActivityItemsController.skip_before_action :verify_authenticity_token
Rails.application.env_config['action_dispatch.show_exceptions']=:all
Rails.application.env_config['action_dispatch.content_security_policy_nonce_generator']=->(_){'NONCE'}
module Ws12InboxTokens
 def form_authenticity_token(form_options: {})
  action,method=form_options.values_at(:action,:method)
  action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
 end
end
ApplicationController.prepend(Ws12InboxTokens)
# Fix generated fixture identities before calling real source writers.
ws12_uuid=0
SecureRandom.define_singleton_method(:uuid){ws12_uuid+=1;format('00000000-0000-4000-8000-%012d',ws12_uuid)}
Random.define_singleton_method(:uuid){SecureRandom.uuid}
root=ENV.fetch('PARITY_WORK'); conn=ActiveRecord::Base.connection
base=JSON.parse(File.read(File.join(root,'vectors/inbox-http.json')))['setup']
base.each{|sql|conn.execute(sql)}
user=User.find(127326141);room=Room.find(486777696)
tables=%w[users rooms memberships messages action_text_rich_texts channel_threads thread_memberships work_thread_events board_sla_nudges saved_items events event_attendances huddle_grants agent_approvals agent_budget_notices sessions two_factor_credentials activity_items sqlite_sequence]
sql_dump=-> do
 tables.flat_map{|table|columns=conn.columns(table).map(&:name);["DELETE FROM #{table}"]+conn.select_all("SELECT * FROM #{table} ORDER BY #{table=='sqlite_sequence' ? 'name' : 'id'}").map{|row|"INSERT INTO #{table}(#{columns.join(',')}) VALUES(#{columns.map{|c|conn.quote(row[c])}.join(',')})"}}
end
saved=sql_dump.call
restore=-> do
 conn.execute('PRAGMA foreign_keys=OFF');conn.execute('DELETE FROM message_search_index');saved.each{|sql|conn.execute(sql)};conn.execute('PRAGMA foreign_keys=ON');ActiveSupport::IsolatedExecutionState.clear
end
labels=JSON.parse(File.read(File.join(root,'parity/.seed/default/labels.json')))
foreign_session=Session.create!(id:9018800103,user:User.find(127326141),user_agent:"Foreign invite",token:"ws12-foreign-session-token")
fresh_session=Session.create!(id:9018800104,user:User.find(149087659),user_agent:"Fresh invite",token:"ws12-fresh-session-token")
session_id=foreign_session.id
# Include these real sessions in the source seed used by each sequence.
saved=sql_dump.call
start=8400000000
only=["DELETE FROM activity_items WHERE id<>#{start}"]
facts=-> do
 {items:ActivityItem.order(:id).map{|i|[i.id,i.user_id,i.event_type,i.read_at&.iso8601(3),i.handled_at&.iso8601(3)]},approvals:AgentApproval.order(:id).map{|a|[a.id,a.status]}}
end
req=->(path,method='get',params={},extra={}){{path:,method:,params:,**extra}}
cases=[]
add=->(id,setup,steps){cases << {id:,setup:,steps:}}
add.call('c001',only+["INSERT INTO activity_items(id,user_id,source_type,source_id,event_type,read_at,created_at,updated_at) VALUES(8400000101,127326141,'Message',935962041,'reply','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')", "INSERT INTO activity_items(id,user_id,source_type,source_id,event_type,read_at,handled_at,created_at,updated_at) VALUES(8400000102,127326141,'Message',935962043,'work_update','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')", "INSERT INTO activity_items(id,user_id,source_type,source_id,event_type,created_at,updated_at) VALUES(8400000103,149087659,'Message',136976342,'mention','2026-03-02 16:00:00','2026-03-02 16:00:00')"],[req.call('/activity/unread_count.json'),req.call('/activity/unread_count.json','get',{},viewer:'jason')])
add.call('c005',[],%w[open read handled].map{|kind|req.call("/activity/8400000099/#{kind}.json",kind=='open' ? 'post' : 'patch',{state:kind})})
add.call('c015',only+["INSERT INTO activity_items(id,user_id,source_type,source_id,event_type,created_at,updated_at) VALUES(8400000101,127326141,'Message',935962041,'reply','2026-02-28 16:00:00','2026-02-28 16:00:00')"],[req.call('/activity.json'),req.call('/activity.json','get',{},sql:["UPDATE activity_items SET updated_at='2026-03-02 16:00:01' WHERE id=8400000101"])])
add.call('c020',only,[req.call("/activity/#{start}/handled",'patch',{state:'handled',status:'read',type:'events'},accept:'text/html'),req.call("/activity/#{start}/read",'patch',{state:'read',status:'unread',type:'bogus'},accept:'text/html')])
add.call('c010',["DELETE FROM activity_items WHERE event_type<>'huddle_started'","UPDATE activity_items SET created_at='2026-03-02 15:57:00' WHERE id=8400000004", "INSERT INTO huddle_grants(id,identity,room_name,session_id,user_id,membership_id,room_id,created_at,updated_at,last_issued_at) VALUES(8300000103,'ws12-foreign','ws12-foreign',#{session_id},127326141,449028908,186869642,'2026-03-02 15:57:00','2026-03-02 15:57:00',NULL)", "INSERT INTO activity_items(id,user_id,source_type,source_id,event_type,created_at,updated_at) VALUES(8400000103,149087659,'HuddleGrant',8300000103,'huddle_started','2026-03-02 15:57:00','2026-03-02 15:57:00')", "UPDATE huddle_grants SET last_issued_at='2026-03-02 15:57:00' WHERE id=8300000003"],[req.call('/activity.json'),req.call('/activity.json','get',{},sql:["UPDATE activity_items SET handled_at='2026-03-02 16:00:00' WHERE id=8400000004", "INSERT INTO huddle_grants(id,identity,room_name,session_id,user_id,membership_id,room_id,created_at,updated_at,last_issued_at) SELECT 8300000104,'fresh-named',room_name,9018800104,user_id,membership_id,room_id,'2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00' FROM huddle_grants WHERE id=8300000003", "INSERT INTO activity_items(id,user_id,source_type,source_id,event_type,created_at,updated_at) VALUES(8400000104,127326141,'HuddleGrant',8300000104,'huddle_started','2026-03-02 16:00:00','2026-03-02 16:00:00')"])])
add.call('c013',["DELETE FROM activity_items WHERE id<>8400000006","UPDATE agent_approvals SET created_at='2026-02-22 16:00:00',expires_at='2026-03-01 16:00:00' WHERE id=1"],[req.call('/activity/unread_count.json'),req.call('/activity.json'),req.call('/activity/unread_count.json')])
# Same exact source classes as the original filter fixture, one source per event type.
restore.call
existing=ActivityItem.order(:id).where(user:).to_a
missing=ActivityItem::EVENT_TYPES-existing.map(&:event_type)-['scheduled_message_dropped']
missing.each_with_index do |type,index|
 source=case type
 when 'event_invitation','event_cancelled','event_reminder'
  Event.create!(room:,organizer:User.find(149087659),title:"Typed #{type}",starts_at:2.days.from_now,time_zone:'UTC')
 when 'huddle_missed'
  original=HuddleGrant.find(8300000003);attrs=original.attributes.except('id');attrs['identity']='ws12-typed-missed';attrs['session_id']=Session.create!(user:User.find(original.user_id),user_agent:'Typed huddle',token:'ws12-typed-session-token').id;attrs['created_at']=3.minutes.ago;attrs['last_issued_at']=3.minutes.ago;HuddleGrant.create!(attrs)
 else
  Message.create!(room:,creator:User.find(149087659),markdown_source:"Typed #{type}",client_message_id:"ws12-typed-#{type}")
 end
 item=ActivityItem.find_or_initialize_by(user:,source:);item.update!(id:8400000200+index,event_type:type)
end
typed=sql_dump.call
filters=ActivityItem::TYPE_FILTER_EVENT_TYPES.keys+['all','bogus']
add.call('c016',typed,filters.map{|type|req.call("/activity.json?type=#{type}")}+[req.call('/activity.json')])
add.call('c017',typed,ActivityItem::TYPE_FILTER_EVENT_TYPES.keys.map{|type|req.call("/activity?type=#{type}",'get',{},accept:'text/html')})
# Keep the production page size (100); the original test's reduced size exercises
# exactly the same full-page cursor / HTML older-link / disjoint follow-up clauses.
restore.call
ActivityItem.delete_all
101.times do |i|
 e=Event.create!(room:,organizer:User.find(149087659),title:"Paged event #{i}",starts_at:2.days.from_now,time_zone:'UTC')
 ActivityItem.find_by!(user:,source:e).update_columns(id:8400001000+i)
end
paged=sql_dump.call
add.call('c018',paged,[req.call('/activity?type=events','get',{},accept:'text/html'),req.call('/activity.json?type=events'),req.call('/activity.json?type=events&before=8400001001')])
rows=cases.map do |entry|
 restore.call
 conn.execute('PRAGMA foreign_keys=OFF');entry[:setup].each{|sql|conn.execute(sql)};conn.execute('PRAGMA foreign_keys=ON')
 browsers={}
 steps=entry[:steps].map do |step|
  Array(step[:sql]).each{|sql|conn.execute(sql)}
  viewer=step[:viewer]||'david';browser=browsers[viewer] ||= ActionDispatch::Integration::Session.new(Rails.application).tap{|b|b.host! 'campfire.test'}
  headers={'Cookie'=>"session_token=#{labels.fetch("session_cookies.#{viewer}")}",'Accept'=>step[:accept]||'application/json','Turbo-Frame'=>'activity_test','User-Agent'=>'Mozilla/5.0 Chrome/140.0.0.0','Content-Type'=>'application/json'}
  if step[:method]=='get'
    browser.get(step[:path],headers:)
  else
    browser.public_send(step[:method],step[:path],params:step[:params].to_json,headers:)
  end
  if %w[c016 c017].include?(entry[:id])
   type=Rack::Utils.parse_query(URI.parse(step[:path]).query.to_s)['type'];types=ActivityItem::TYPE_FILTER_EVENT_TYPES[type]
   selected=ActivityItem.where(user:);selected=selected.where(event_type:types) if types
   raise 'type fixture must return success' unless browser.response.status==200
   if entry[:id]=='c016'
    payload=JSON.parse(browser.response.body);returned=payload.fetch('activity_items')
    raise 'wrong type normalization' unless payload.fetch('type_filter')==(types ? type : 'all')
    raise 'type fixture missing declared event' if types && returned.map{|r|r['event_type']}.uniq.sort!=types.sort
    raise 'type fixture wrong returned count' unless returned.size==selected.count
   else
    html=Nokogiri::HTML(browser.response.body)
    ActivityItem.where(user:).each{|item|count=html.css("#activity_item_#{item.id}").size;raise 'HTML type membership mismatch' unless count==(selected.exists?(id:item.id) ? 1 : 0)}
   end
  end
  {**step,status:browser.response.status,body:browser.response.body,headers:%w[content-type cache-control pragma location].to_h{|k|[k,browser.response.headers[k]]},facts:facts.call}
 end
 {id:entry[:id],setup:saved+entry[:setup],steps:}
end
sources=%w[app/controllers/activity_items_controller.rb test/controllers/activity_items_controller_test.rb].to_h{|p|[p,Digest::SHA256.file(Rails.root.join(p)).hexdigest]}
puts JSON.pretty_generate(sources:,rows:)
warn "WS12_INBOX_REMAINING_RAILS #{rows.size} named HTTP sequences; #{rows.sum{|r|r[:steps].size}} full responses and saved facts; 0 masks"
