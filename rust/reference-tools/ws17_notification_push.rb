require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
ActiveRecord::Base.logger=nil
ActiveJob::Base.queue_adapter=:test
rows=[]
conn=ActiveRecord::Base.connection
user=User.find_by!(email_address:"david@37signals.com")
recipient=User.find_by!(email_address:"jason@37signals.com")
other=User.find_by!(email_address:"jz@37signals.com")
room=Room.find_by!(name:"Designers")
direct=Room.find(ActiveRecord::FixtureSet.identify(:david_and_jason))
event=Event.find(ActiveRecord::FixtureSet.identify(:launch_party))
def insert_sql(record)
 c=ActiveRecord::Base.connection;a=record.attributes_for_database
 "INSERT OR REPLACE INTO #{c.quote_table_name(record.class.table_name)} (#{a.keys.map { |key|c.quote_column_name(key) }.join(',')}) VALUES (#{a.values.map { |value|c.quote(value) }.join(',')});"
end
travel_to(Time.utc(2026,3,2,16)) do
 board=Rooms::Board.create_for({name:"Launch",creator:user},users:[user,recipient])
 post=ChannelThread.create_board_post!(room:board,creator:user,name:"Stale work",work_status:"in_progress",owner_id:recipient.id)
 nudge=BoardSlaNudge.create!(room:board,channel_thread:post,work_status:"in_progress",stage:"nudge",status_entered_at:2.hours.ago,recipient:)
 ENV["LIVEKIT_API_SECRET"]="ws17-parity-huddle-source"
 grant=HuddleGrant.issue!(session:Session.find(ActiveRecord::FixtureSet.identify(:david_safari)),membership:direct.memberships.find_by!(user:))
 base_members=[room.memberships.find_by!(user:),room.memberships.find_by!(user:recipient),direct.memberships.find_by!(user:recipient)]
 base_sql=base_members.map { |m|insert_sql(m) }
 board_sql=[board,post,*board.memberships,nudge].map { |r|insert_sql(r) }
 roles=[user,recipient,other].map { |u|[u.id,u.role_before_type_cast] }.to_h
 subscriptions=Push::Subscription.where(user_id:[user.id,recipient.id,other.id]).map { |s|insert_sql(s) }
 run=lambda do |name,kind,custom=[]|
  setup=base_sql.dup+subscriptions
  roles.each { |id,role|setup << "UPDATE users SET status=0,role=#{role},dnd_enabled=0,dnd_until=NULL,quiet_hours_enabled=0,quiet_hours_start_minute=540,quiet_hours_end_minute=1020,time_zone='UTC',meeting_status_enabled=0,meeting_dnd_enabled=0,ooo_until=NULL,ooo_notify_enabled=0,inbox_preferences='{}' WHERE id=#{id};" }
  setup << "DELETE FROM dnd_allowed_users WHERE user_id IN (#{user.id},#{recipient.id},#{other.id});"
  setup << "DELETE FROM calendar_meeting_caches WHERE user_id IN (#{user.id},#{recipient.id},#{other.id});"
  setup << "UPDATE memberships SET involvement='mentions',connected_at=NULL,last_huddle_join_push_at=NULL WHERE room_id IN (#{room.id},#{direct.id});"
  setup << "UPDATE events SET room_id=#{room.id},organizer_id=#{user.id},title='Launch party planning',starts_at='2026-03-02 16:15:00',ends_at='2026-03-02 17:15:00',venue_room_id=NULL WHERE id=#{event.id};"
  setup += board_sql if kind=="board"
  setup += custom
  setup.each { |sql|conn.execute(sql) }
  deliveries=[]
  pool=Object.new
  pool.define_singleton_method(:queue) do |payload,subs|
   subs=subs.order(:id).to_a
   encoded=subs.map { |sub|WebPush::Notification.new(**payload.reverse_merge(tag:nil),badge:sub.user.memberships.unread.count,endpoint:nil,endpoint_ip_resolver:nil,p256dh_key:nil,auth_key:nil).send(:encoded_message) }
   deliveries << {payload:payload.reverse_merge(tag:nil),subscriptions:subs.map(&:id),users:subs.map(&:user_id),encoded:}
  end
  old=Rails.configuration.x.web_push_pool;Rails.configuration.x.web_push_pool=pool
  recipient.reload;user.reload
  case kind
  when "event";Event::ReminderPusher.new(event:event.reload).push
  when "board";BoardAutomations::NudgePusher.new(nudge:nudge.reload).push
  when "join";Huddle::JoinPusher.new(grant:grant.reload,recipient:recipient.reload,room_membership:direct.memberships.find_by(user:recipient)).push
  when "invitation"
   item=ActivityItem.new(user:recipient,source:grant,event_type:"huddle_started")
   Huddle::InvitationPusher.new(activity_item:item).push
  end
  Rails.configuration.x.web_push_pool=old
  throttle=direct.memberships.find_by(user:recipient)&.last_huddle_join_push_at
  rows << {name:,kind:,setup_sql:setup.join("\n"),event_id:event.id,nudge_id:nudge.id,room_id:direct.id,recipient_id:recipient.id,sender_id:user.id,deliveries:,throttle:throttle&.iso8601(6)}
 end
 run.call("event_attendees_still_members","event",["DELETE FROM memberships WHERE user_id=#{recipient.id} AND room_id=#{room.id};"])
 run.call("event_inbox_switch_ignored","event",["UPDATE users SET inbox_preferences='{\"event_reminders\":false}' WHERE id=#{user.id};"])
 run.call("event_venue","event",["UPDATE events SET venue_room_id=#{room.id} WHERE id=#{event.id};"])
 run.call("event_direct_organizer_title","event",["UPDATE events SET room_id=#{direct.id} WHERE id=#{event.id};"])
 [["three_minutes",180],["singular",80],["starting_now",29],["round_30",30],["round_90",90],["recently_started",-240],["stale_boundary",-300],["stale",-1800]].each { |name,seconds|run.call("event_#{name}","event",["UPDATE events SET starts_at=#{conn.quote(Time.current+seconds)} WHERE id=#{event.id};"]) }
 run.call("event_ended","event",["UPDATE events SET ends_at=#{conn.quote(Time.current)} WHERE id=#{event.id};"])
 run.call("event_dnd_no_sender_exception","event",["UPDATE users SET dnd_enabled=1 WHERE id IN (#{user.id},#{recipient.id});","INSERT INTO dnd_allowed_users(user_id,allowed_user_id,created_at,updated_at) VALUES (#{recipient.id},#{user.id},#{conn.quote(Time.current)},#{conn.quote(Time.current)});"])
 run.call("event_members_nothing_still_push","event",["UPDATE memberships SET involvement='nothing' WHERE room_id=#{room.id};"])
 run.call("event_inactive_and_bot_excluded","event",["UPDATE users SET status=1 WHERE id=#{user.id};","UPDATE users SET role=2 WHERE id=#{recipient.id};"])
 run.call("board_nudge","board")
 run.call("board_escalation","board",["UPDATE board_sla_nudges SET stage='escalation',recipient_id=#{user.id} WHERE id=#{nudge.id};"])
 run.call("board_left","board",["DELETE FROM memberships WHERE room_id=#{board.id} AND user_id=#{recipient.id};"])
 run.call("board_dnd","board",["UPDATE users SET dnd_enabled=1 WHERE id=#{recipient.id};"])
 run.call("board_hidden_still_member","board",["UPDATE memberships SET involvement='invisible',connected_at=#{conn.quote(Time.current)} WHERE room_id=#{board.id} AND user_id=#{recipient.id};"])
 run.call("board_unknown_status","board",["UPDATE board_sla_nudges SET work_status='waiting_for_review' WHERE id=#{nudge.id};"])
 run.call("join_baseline","join")
 [0,600,660].each { |seconds|run.call("join_throttled_#{seconds}","join",["UPDATE memberships SET last_huddle_join_push_at=#{conn.quote(Time.current-seconds)} WHERE room_id=#{direct.id} AND user_id=#{recipient.id};"]) }
 [["dnd","dnd_enabled=1"],["quiet_hours","quiet_hours_enabled=1"],["ooo","ooo_until='2026-03-03 16:00:00'"],["ooo_notifications","ooo_until='2026-03-03 16:00:00',ooo_notify_enabled=1"],["inbox_off","inbox_preferences='{\"huddle_invitations\":false}'"],["inbox_string_false","inbox_preferences='{\"huddle_invitations\":\"false\"}'"]].each { |name,attrs|run.call("join_#{name}","join",["UPDATE users SET #{attrs} WHERE id=#{recipient.id};"]) }
 run.call("join_starred_dnd","join",["UPDATE users SET dnd_enabled=1 WHERE id=#{recipient.id};","INSERT INTO dnd_allowed_users(user_id,allowed_user_id,created_at,updated_at) VALUES (#{recipient.id},#{user.id},#{conn.quote(Time.current)},#{conn.quote(Time.current)});"])
 run.call("join_meeting_quiet","join",["UPDATE users SET meeting_status_enabled=1,meeting_dnd_enabled=1 WHERE id=#{recipient.id};","INSERT INTO calendar_meeting_caches(user_id,busy_intervals,fetched_at,created_at,updated_at) VALUES (#{recipient.id},'[[\"2026-03-02T15:55:00Z\",\"2026-03-02T16:55:00Z\"]]',#{conn.quote(Time.current)},#{conn.quote(Time.current)},#{conn.quote(Time.current)});"])
 ["nothing","invisible","muted",nil].each { |mode|run.call("join_mode_#{mode||'null'}","join",["UPDATE memberships SET involvement=#{conn.quote(mode)} WHERE room_id=#{direct.id} AND user_id=#{recipient.id};"]) }
 [0,60,61].each { |seconds|run.call("join_connected_#{seconds}","join",["UPDATE memberships SET connected_at=#{conn.quote(Time.current-seconds)} WHERE room_id=#{direct.id} AND user_id=#{recipient.id};"]) }
 run.call("join_no_subscriptions","join",["DELETE FROM push_subscriptions WHERE user_id=#{recipient.id};"])
 run.call("invitation_baseline","invitation")
 run.call("invitation_dnd","invitation",["UPDATE users SET dnd_enabled=1 WHERE id=#{recipient.id};"])
 run.call("invitation_starred_dnd","invitation",["UPDATE users SET dnd_enabled=1 WHERE id=#{recipient.id};","INSERT INTO dnd_allowed_users(user_id,allowed_user_id,created_at,updated_at) VALUES (#{recipient.id},#{user.id},#{conn.quote(Time.current)},#{conn.quote(Time.current)});"])
 ["nothing","invisible","muted",nil].each { |mode|run.call("invitation_mode_#{mode||'null'}","invitation",["UPDATE memberships SET involvement=#{conn.quote(mode)} WHERE room_id=#{direct.id} AND user_id=#{recipient.id};"]) }
 run.call("invitation_connected","invitation",["UPDATE memberships SET connected_at=#{conn.quote(Time.current)} WHERE room_id=#{direct.id} AND user_id=#{recipient.id};"])
end
puts JSON.generate(reference:"d7c7de92",now:"2026-03-02T16:00:00Z",rows:)
