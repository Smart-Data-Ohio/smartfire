require_relative "ws8_vector_helpers"
ENV.delete("LIVEKIT_INTERNAL_URL")
before=dump
user=User.find(ActiveRecord::FixtureSet.identify("david"))
room=Room.find(ActiveRecord::FixtureSet.identify("watercooler"))
membership=room.memberships.find_by!(user:user)
agent=Agent.first!
# Include exact boundaries, one microsecond older/newer, and unread/pending exceptions.
[-1,0,1].each do |offset|
  AgentEvent.create!(agent:agent,event_type:"mention",outcome:"acknowledged",created_at:90.days.ago+(offset/1_000_000r))
  Github::WebhookDelivery.create!(delivery_guid:"ws8-#{offset}",created_at:14.days.ago+(offset/1_000_000r))
  HuddleCleanup.create!(operation: :delete_room,room_name:"ws8-done-#{offset}",completed_at:7.days.ago+(offset/1_000_000r))
  travel_to(1.year.ago+(offset/1_000_000r), with_usec: true) { AuditLog.record!(action:"user.ban",target:user) }
  card=Fizzy::Card.create!(account_id:"ws8",number:offset+2)
  Fizzy::CardCache.create!(user:user,card:card,updated_at:1.day.ago+(offset/1_000_000r))
  grant=HuddleGrant.create!(room:room,membership:membership,user:user,session:user.sessions.first!,identity:"ws8-retention-#{offset}",room_name:"ws8-retention-room",revoked_at:30.days.ago+(offset/1_000_000r))
  HuddleCleanup.create!(operation: :remove_participant,huddle_grant:grant,room_name:grant.room_name,identity:grant.identity)
  ActivityItem.create!(user:user,source:grant,event_type:"huddle_missed")
end
[-1,0,1].each_with_index do |offset,i|
  u=User.find(ActiveRecord::FixtureSet.identify(%w[jason kevin jz][i]))
  item=ActivityItem.create!(user:u,source:Message.find(ActiveRecord::FixtureSet.identify("first")),event_type:"mention")
  item.update_columns(read_at:180.days.ago,updated_at:180.days.ago+(offset/1_000_000r))
  dev,_=TwoFactorRememberedDevice.create_for!(u,user_agent:"ws8",ip_address:"192.0.2.1")
  dev.update!(expires_at:Time.current+(offset/1_000_000r))
  session=u.sessions.create!(user_agent:"ws8")
  secret=TwoFactorSetupSecret.issue_for!(session)
  secret.update!(expires_at:Time.current+(offset/1_000_000r))
end
unread=ActivityItem.create!(user:user,source:Message.find(ActiveRecord::FixtureSet.identify("first")),event_type:"mention")
unread.update_columns(updated_at:400.days.ago)
HuddleCleanup.create!(operation: :delete_room,room_name:"ws8-pending",created_at:400.days.ago)
HuddleGrant.create!(room:room,membership:membership,user:user,session:user.sessions.first!,identity:"ws8-active",room_name:"ws8-retention-room",created_at:400.days.ago)
stuck=Rooms::Closed.create_for({name:"ws8-stuck",creator:user},users:[user])
stuck.begin_destroy!
stuck.update_columns(deleted_at:61.minutes.ago,destroy_enqueued_at:nil)
setup=delta(before)
ActiveJob::Base.queue_adapter.enqueued_jobs.clear
Retention::PruneJob.perform_now
# Compare every survivor's id, including fixtures, so unexpected deletions are caught too.
tables=%w[agent_events two_factor_remembered_devices two_factor_setup_secrets activity_items github_webhook_deliveries huddle_cleanups audit_logs fizzy_card_caches huddle_grants]
checks=tables.map { |t| check("SELECT id FROM #{t} ORDER BY id") }
checks << check("SELECT id,huddle_grant_id,room_name,identity FROM huddle_cleanups WHERE room_name='ws8-retention-room' ORDER BY id")
checks << check("SELECT destroy_enqueued_at FROM rooms WHERE id=#{stuck.id}")
jobs=ActiveJob::Base.queue_adapter.enqueued_jobs.select { |j| j[:job]==Room::DestroyJob }.map { |j| j[:args].first }
File.write(ARGV.fetch(0),JSON.pretty_generate({now:Time.current.to_fs(:db),setup_sql:setup,checks:checks,room_jobs:jobs,audit_cutoff:1.year.ago.to_fs(:db)})+"\n")
puts "WS8 retention vectors: #{tables.size} row kinds, #{checks.size} survivor checks, #{jobs.size} stuck room jobs"
