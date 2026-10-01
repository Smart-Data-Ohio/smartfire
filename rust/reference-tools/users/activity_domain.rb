# Activity access and state vectors from our pinned Rails models, without render normalization.
require "json"
require "digest"
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
Rails.logger = ActiveSupport::Logger.new($stderr)
hashes = JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "reference-tools/users/activity-domain-source-hashes.json")))
hashes.each { |file, hash| raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest == hash }
users = [127326141,149087659,712064548,394959859].map { |id| User.find(id) }
snapshots = []
snapshot = ->(name) do
  snapshots << {name: name, viewers: users.map do |user|
    scope = ActivityItem.accessible_to(user)
    {user_id:user.id,ids:scope.ordered.pluck(:id),unread_count:scope.unread.count,
     filters:%w[unread read handled].to_h { |state| [state,scope.public_send(state).ordered.pluck(:id)] },
     types:ActivityItem::TYPE_FILTERS.keys.to_h { |type| [type,scope.with_type_filter(type).ordered.pluck(:id)] }}
  end}
end
snapshot.call("baseline")
Membership.where(user_id:users.first.id).delete_all
snapshot.call("membership_revoked")
users.first.update_columns(status:1)
snapshot.call("inactive")
users.first.update_columns(status:0)
# Source identity alone is insufficient for a foreign session/security source.
ActivityItem.where(user:users.first).delete_all
source = users[1].sessions.first || users[1].sessions.create!(ip_address:"127.0.0.1")
ActivityItem.create!(user:users.first,source:source,event_type:"new_sign_in")
snapshot.call("foreign_session")
ActivityItem.where(user:users.first).delete_all
source = Message.order(:id).first
item = ActivityItem.create!(user:users.first,source:source,event_type:"mention")
states = []
%w[initial read read handled handled unhandled unhandled unread unread].each_with_index do |state, index|
  # Freeze each step at a distinct second except repeated operations: no-op saves must stay still.
  at = Time.utc(2026,3,2,16,0,0) + index / 2
  travel_to(at) do
    case state
    when "read" then item.mark_read!
    when "handled" then item.mark_handled!
    when "unhandled" then item.mark_unhandled!
    when "unread" then item.mark_unread!
    end
  end
  states << {operation:state,now:at.to_i,state:item.state,read_at:item.read_at&.to_i,
    handled_at:item.handled_at&.to_i,updated_at:item.updated_at.to_i}
end
baseline_snapshots = snapshots
snapshots = []
Membership.delete_all
board = Room.boards.first
thread = board.channel_threads.first
agent = Agent.first
huddle_member = board.memberships.create!(user:users[1],involvement:"everything")
missing = [
  WorkThreadEvent.create!(id:8300000001,thread:thread,event_type:"work_update",to_status:"planned"),
  BoardSlaNudge.create!(id:8300000002,room:board,channel_thread:thread,recipient:users.first,
    work_status:"planned",stage:"nudge",status_entered_at:Time.current),
  HuddleGrant.create!(id:8300000003,room:board,user:users[1],membership:huddle_member,
    session:users[1].sessions.first,identity:"ws12-access-fixture",room_name:"ws12-board",last_issued_at:Time.current),
  AgentBudgetNotice.create!(id:8300000004,agent:agent,cap:"messages",day:Date.current)
]
setup = missing.map { |source| {table:source.class.table_name,row:source.class.connection.select_all("SELECT * FROM #{source.class.quoted_table_name} WHERE id=#{source.id}").first} }
# Restore the fixture memberships directly from the snapshot input for a controlled access matrix.
[board.id,Message.first.room_id,SavedItem.first.message.room_id,Event.first.room_id,HuddleGrant.first.room_id].compact.uniq.each do |room_id|
  users.each { |user| Membership.find_or_create_by!(room_id:room_id,user:user) { |member| member.involvement="everything" } }
end
ActivityItem.delete_all
sources = [Message.first,SavedItem.first,missing[0],missing[1],missing[2],Event.first,AgentApproval.first,missing[3],ScheduledMessage.first,TwoFactorCredential.first,users.first.sessions.first]
types = %w[mention message_reminder work_update work_sla huddle_started event_update agent_approval_request agent_budget_exceeded scheduled_message_dropped two_factor_lockout new_sign_in]
sources.zip(types).each_with_index do |(source,event_type),index|
  users.each_with_index do |user,recipient|
    ActivityItem.create!(id:8400000000+index*10+recipient,user:user,source:source,event_type:event_type)
  end
end
users.each_with_index do |user,index|
  ActivityItem.insert_all!([{id:8500000000+index,user_id:user.id,source_type:"UnknownSource",source_id:1,event_type:"mention",created_at:Time.current,updated_at:Time.current}])
  ActivityItem.insert_all!([{id:8500000010+index,user_id:user.id,source_type:"Message",source_id:-1,event_type:"mention",created_at:Time.current,updated_at:Time.current}])
end
rows = ActivityItem.connection.select_all("SELECT * FROM activity_items ORDER BY id").to_a
memberships = Membership.connection.select_all("SELECT * FROM memberships ORDER BY id").to_a
snapshot.call("all_sources")
Membership.where(user_id:users.first.id).delete_all
snapshot.call("all_sources_membership_revoked")
agent.update_columns(owner_id:users[2].id)
snapshot.call("agent_reassigned")
agent.user.update_columns(status:1)
snapshot.call("agent_inactive")
matrix = {setup:setup,memberships:memberships,items:rows,agent_id:agent.id,agent_user_id:agent.user_id,snapshots:snapshots}
puts JSON.pretty_generate(reference:"d7c7de92",snapshots:baseline_snapshots,states:states,matrix:matrix)
warn "Rails activity domain oracle: #{baseline_snapshots.size + snapshots.size} access snapshots over all 11 source types, #{states.size} state transitions; reference d7c7de92"
