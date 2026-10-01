require 'json'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
user=User.find_by!(email_address:'david@37signals.com');Current.user=user
room=user.rooms.find_by!(name:'All Talk')
message=room.messages.create!(creator:user,markdown_source:'Reminder @[Jason] & '+('長'*180),client_message_id:'reminder-push-source')
item=SavedItem.create!(user:,message:)
base={dnd_enabled:false,dnd_until:nil,presence_setting:'auto',quiet_hours_enabled:false,quiet_hours_start_minute:nil,quiet_hours_end_minute:nil,time_zone:'America/New_York',meeting_status_enabled:false,meeting_dnd_enabled:false,ooo_until:nil,ooo_calendar_enabled:false,ooo_notify_enabled:false}
now=Time.current
busy=[[now.iso8601,(now+1.hour).iso8601]]
ended=[[(now-1.hour).iso8601,now.iso8601]]
cases=[{}, {dnd_enabled:true}, {dnd_enabled:true,dnd_until:now+1.second}, {dnd_enabled:true,dnd_until:now}, {dnd_enabled:true,dnd_until:now-1.second}, {presence_setting:'dnd'}, {presence_setting:'invisible'}, {quiet_hours_enabled:true,quiet_hours_start_minute:660,quiet_hours_end_minute:720}, {quiet_hours_enabled:true,quiet_hours_start_minute:600,quiet_hours_end_minute:660}, {quiet_hours_enabled:true,quiet_hours_start_minute:660,quiet_hours_end_minute:660}, {quiet_hours_enabled:true,quiet_hours_start_minute:1380,quiet_hours_end_minute:420,time_zone:'Hawaii'}, {quiet_hours_enabled:true,quiet_hours_start_minute:1380,quiet_hours_end_minute:420}, {quiet_hours_enabled:true,quiet_hours_start_minute:nil,quiet_hours_end_minute:720}, {quiet_hours_enabled:false,quiet_hours_start_minute:660,quiet_hours_end_minute:720}, {meeting_status_enabled:true,meeting_dnd_enabled:true}, {meeting_status_enabled:false,meeting_dnd_enabled:true}, {meeting_status_enabled:true,meeting_dnd_enabled:false}, {ooo_until:now+1.hour}, {ooo_until:now}, {ooo_until:now+1.hour,ooo_notify_enabled:true}, {ooo_calendar_enabled:true}, {ooo_calendar_enabled:true,ooo_notify_enabled:true}, {ooo_calendar_enabled:false}, {meeting_status_enabled:true,meeting_dnd_enabled:true}, {ooo_calendar_enabled:true}, {meeting_status_enabled:true,meeting_dnd_enabled:true}, {dnd_enabled:true,ooo_notify_enabled:true}]
vectors=cases.map.with_index do |changes,i|
 attrs=base.merge(changes)
 intervals= [15,23].include?(i) ? ended : (i==25 ? ['bad',[nil,'bad'],[now.iso8601,nil]] : busy)
 cache=user.meeting_cache||user.create_meeting_cache!
 cache.update!(busy_intervals:intervals,ooo_intervals:intervals)
 user.update_columns(attrs);user.reload
 {attrs:user.attributes.slice(*base.keys.map(&:to_s)),intervals:,allowed:Notifications::Policy.new(recipient:user,kind: :reminder,now:).push?}
end
user.update_columns(base);user.reload
pool=Object.new
pool.instance_variable_set(:@calls,[])
def pool.queue(payload,subscriptions);@calls<<{payload:,subscription_ids:subscriptions.order(:id).pluck(:id)};end
def pool.calls;@calls;end
def pool.shutdown;end
Rails.configuration.x.web_push_pool=pool
SavedItem::ReminderPushJob.perform_now(item)
payload=pool.calls.last
room.update_columns(name:'😀'*80)
SavedItem::ReminderPushJob.perform_now(item)
long_title_payload=pool.calls.last
room.update_columns(name:'All Talk')
rows={'messages'=>ActiveRecord::Base.connection.select_all("SELECT * FROM messages WHERE id=#{message.id}").to_a,'action_text_rich_texts'=>ActiveRecord::Base.connection.select_all("SELECT * FROM action_text_rich_texts WHERE record_type='Message' AND record_id=#{message.id}").to_a,'saved_items'=>ActiveRecord::Base.connection.select_all("SELECT * FROM saved_items WHERE id=#{item.id}").to_a}
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',vectors:,payload:,long_title_payload:,rows:,saved_item_id:item.id)+"\n")
puts "WS8bm2 reminder push Rails oracle: #{vectors.size} policy cases; 2 captured real job payload/subscription handoffs; 3 fixture tables"
