# Real Rails writes and callbacks are the oracle. The setup delta is replayed by Rust.
require_relative "ws8_vector_helpers"
ENV.delete("LIVEKIT_INTERNAL_URL")
ENV["LIVEKIT_API_KEY"] = "fixture-key"
ENV["LIVEKIT_API_SECRET"] = "fixture-secret"
before = dump
user = User.find(ActiveRecord::FixtureSet.identify("david"))
other = User.find(ActiveRecord::FixtureSet.identify("jason"))
room = Rooms::Stage.create_for({name: "Destroy oracle", creator: user}, users: [user, other])
root = room.messages.create!(creator: user, markdown_source: "Root")
thread = ChannelThread.create!(room: room, creator: user, name: "Discussion")
thread.post_message!(creator: other, attributes: {markdown_source: "Reply"})
work = thread.work_thread_events.create!(actor: user, event_type: "work_update", to_status: "planned")
ActivityItem.create!(user: other, source: work, event_type: "work_update")
nudge = BoardSlaNudge.create!(room:room,channel_thread:thread,recipient:other,work_status:"planned",stage:"nudge",status_entered_at:1.hour.ago)
ActivityItem.create!(user:other,source:nudge,event_type:"work_sla")
scheduled = ScheduledMessage.create!(user: user, room: room, thread: thread, markdown_source: "Pending", send_at: 1.hour.from_now)
scheduled.drop!(reason: "fixture")
event = room.events.create!(organizer: user, title: "Meeting", starts_at: 2.days.from_now, time_zone: "UTC")
EventCalendarEntry.create!(event: event, user: user, google_event_id: "ws8-calendar-copy")
EventReference.create!(message: root, event: event)
subscription = Github::RepositorySubscription.create!(room: room, owner: "rails", repo: "rails", created_by: user)
Github::Notification.create!(subscription: subscription, dedupe_key: "ws8-delete")
membership = room.memberships.find_by!(user: user)
Stream.create!(room: room, membership: membership, user: user, quality: "1080p15")
grant = HuddleGrant.create!(room: room, membership: membership, user: user, session: user.sessions.first!, identity: "ws8-participant", room_name: "ws8-livekit-room")
HuddleCleanup.create!(operation: :remove_participant, huddle_grant: grant, room_name: grant.room_name, identity: grant.identity)
ActivityItem.create!(user: user, source: grant, event_type: "huddle_missed")
agent = Agent.first!
AgentGrant.create!(agent: agent, granted_by: user, room: room, capability: "read_messages")
ledger = agent.agent_events.create!(room: room, event_type: "mention", outcome: "pending")
approval = AgentApproval.create!(agent: agent, room: room, action: "github.comment", summary: "Preserved", expires_at: 1.hour.from_now)
setup = delta(before)
ActiveJob::Base.queue_adapter.enqueued_jobs.clear
room.begin_destroy!
begun = [check("SELECT deleted_at,direct_member_key FROM rooms WHERE id=#{room.id}"), check("SELECT id FROM memberships WHERE room_id=#{room.id}"), check("SELECT revoked_at,updated_at FROM huddle_grants WHERE id=#{grant.id}"), check("SELECT revoked_at,updated_at FROM agent_grants WHERE room_id=#{room.id}"), check("SELECT ended_at,updated_at FROM streams WHERE room_id=#{room.id}"), check("SELECT operation,room_name,huddle_grant_id FROM huddle_cleanups WHERE room_name='ws8-livekit-room' ORDER BY id")]
fail_final = -> { raise "ws8 final delete failure" if id == room.id }
Room.set_callback(:destroy, :before, fail_final)
begin
  Room::DestroyJob.perform_now(room.id)
rescue RuntimeError => error
  raise unless error.message == "ws8 final delete failure"
ensure
  Room.skip_callback(:destroy, :before, fail_final)
end
# Capture before the final retry: child records are gone, Room#destroy dependencies rolled back.
retry_tables = %w[memberships messages channel_threads scheduled_messages events huddle_grants streams github_repository_subscriptions agent_grants]
progress = retry_tables.map { |t| check("SELECT id FROM #{t} WHERE room_id=#{room.id}") }
progress += [check("SELECT id FROM rooms WHERE id=#{room.id}"),check("SELECT room_id FROM agent_events WHERE id=#{ledger.id}"),check("SELECT room_id FROM agent_approvals WHERE id=#{approval.id}")]
Room::DestroyJob.perform_now(room.id)
tables = %w[memberships messages channel_threads scheduled_messages events huddle_grants streams github_repository_subscriptions agent_grants message_pins board_tag_assignments board_sla_rules board_sla_nudges board_stale_digests]
finished = tables.map { |t| check("SELECT id FROM #{t} WHERE room_id=#{room.id}") }
finished += [check("SELECT id FROM rooms WHERE id=#{room.id}"), check("SELECT room_id FROM agent_events WHERE id=#{ledger.id}"), check("SELECT room_id FROM agent_approvals WHERE id=#{approval.id}"), check("SELECT huddle_grant_id,room_name,identity FROM huddle_cleanups WHERE room_name IN ('ws8-livekit-room','#{Huddle.room_name(room.id)}') ORDER BY id"), check("SELECT id FROM event_calendar_entries WHERE event_id=#{event.id}"), check("SELECT id FROM event_attendances WHERE event_id=#{event.id}"), check("SELECT id FROM github_notifications WHERE subscription_id=#{subscription.id}"), check("SELECT id FROM activity_items WHERE (source_type='ScheduledMessage' AND source_id=#{scheduled.id}) OR (source_type='HuddleGrant' AND source_id=#{grant.id}) OR (source_type='WorkThreadEvent' AND source_id=#{work.id}) OR (source_type='Event' AND source_id=#{event.id}) OR (source_type='BoardSlaNudge' AND source_id=#{nudge.id})")]
calendar_jobs = ActiveJob::Base.queue_adapter.enqueued_jobs.select { |j| j[:job] == Calendar::RemoteDeleteJob }.map { |j| j[:args] }
result = {now: Time.current.to_fs(:db), room_id: room.id, setup_sql: setup, begun: begun, finished: finished, calendar_jobs: calendar_jobs, progress: progress}
ActiveRecord::FixtureSet.reset_cache
load File.join(__dir__, "load_fixtures.rb")
before = dump
live = Rooms::Closed.create_for({name: "Live", creator: user}, users: [user])
result[:live] = {now: Time.current.to_fs(:db), setup_sql: delta(before), room_id: live.id, checks: [check("SELECT id,deleted_at,destroy_enqueued_at FROM rooms WHERE id=#{live.id}")]}
# Boundaries are strict: deleted_at == cutoff and claim == cutoff remain held.
%w[unclaimed expired held boundary_deleted boundary_claim].each do |label|
  r = Rooms::Closed.create_for({name: label, creator: user}, users: [user])
  r.begin_destroy!
  r.update_columns(deleted_at: (label == "boundary_deleted" ? 10.minutes.ago : 11.minutes.ago), destroy_enqueued_at: {"unclaimed" => nil, "expired" => 11.minutes.ago, "held" => 9.minutes.ago, "boundary_claim" => 10.minutes.ago}[label])
end
recovery_setup = delta(before)
ActiveJob::Base.queue_adapter.enqueued_jobs.clear
Room::DestroyJob.reenqueue_stuck!
recovery_ids = ActiveJob::Base.queue_adapter.enqueued_jobs.select { |j| j[:job] == Room::DestroyJob }.map { |j| j[:args].first }
first = check("SELECT id,deleted_at,destroy_enqueued_at FROM rooms WHERE name IN ('unclaimed','expired','held','boundary_deleted','boundary_claim') ORDER BY id")
ActiveJob::Base.queue_adapter.enqueued_jobs.clear
Room::DestroyJob.reenqueue_stuck!
repeat_ids = ActiveJob::Base.queue_adapter.enqueued_jobs.select { |j| j[:job] == Room::DestroyJob }.map { |j| j[:args].first }
result[:recovery] = {now: Time.current.to_fs(:db), setup_sql: recovery_setup, ids: recovery_ids, checks: [first], repeat_ids: repeat_ids}
result[:readers] = {all: C.select_values("SELECT id FROM rooms WHERE deleted_at IS NULL ORDER BY id"), closed: C.select_values("SELECT id FROM rooms WHERE deleted_at IS NULL AND type='Rooms::Closed' ORDER BY id"), original: Room.alive.original.id}
travel_to(Time.current+601, with_usec: true) do
  ActiveJob::Base.queue_adapter.enqueued_jobs.clear
  Room::DestroyJob.reenqueue_stuck!
  result[:recovery][:expired_claim_count] = ActiveJob::Base.queue_adapter.enqueued_jobs.count { |j| j[:job] == Room::DestroyJob }
end
result[:huddle_secret] = ENV.fetch("LIVEKIT_API_SECRET")
result[:huddle_name] = Huddle.room_name(room.id)
File.write(ARGV.fetch(0), JSON.pretty_generate(result) + "\n")
puts "WS8 deletion vectors: #{begun.size} begin checks, #{finished.size} cascade checks, #{calendar_jobs.size} calendar jobs, #{recovery_ids.size} recovery claims"
