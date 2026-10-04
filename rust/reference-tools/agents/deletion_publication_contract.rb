# Independent PR #176 cursor probe: real deletion callbacks on the current Rails reference pin.
require 'active_support/testing/time_helpers'
require 'timeout'
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026, 3, 2, 16)
agent = Agent.find(773018776)
room = Room.find(486777696)
human = User.find(127326141)
AgentGrant.delete_all
room.memberships.grant_to([agent.user])
Webhook.find_or_create_by!(user:agent.user) { |hook| hook.url='https://bots.example.test/hook' }

# Observe or pause immediately before the real after_destroy_commit ledger insert.
ChannelThread.prepend(Module.new do
  def emit_deleted_work_unassigned
    Thread.current[:ws11_before_deletion_publication]&.call
    super
  end
end)
poll = ->(since) { Agents::EventPolling.poll(agent:agent, since:since, limit:50, presenter:->(message) {{id:message.id}}) }
types = ->(page) { page[:events].map { |event| event[:event_type] } }
make_thread = ->(name) do
  thread=ChannelThread.create!(room:room, creator:human, name:name, work_status:'planned')
  thread.update_columns(work_owner_id:agent.user_id)
  thread
end
make_event = ->(kind) { agent.agent_events.create!(event_type:kind, room:room, outcome:'delivered', metadata:{status:'completed'}) }
result={}

agent.agent_events.delete_all
thread=make_thread.call('Same transaction publication')
since=AgentEvent.maximum(:id).to_i
during=nil
later=nil
Thread.current[:ws11_before_deletion_publication]=-> { during=poll.call(since) }
begin
  ActiveRecord::Base.transaction do
    thread.destroy!
    later=make_event.call('github_action_completed')
  end
ensure
  Thread.current[:ws11_before_deletion_publication]=nil
end
deleted=agent.agent_events.where(event_type:'work_unassigned').sole
resumed=poll.call(during[:next_since])
result[:same_transaction]={first_types:types.call(during), first_cursor_is_committed_event:during[:next_since]==later.id,
  deletion_id_after_committed_event:deleted.id>later.id, resumed_types:types.call(resumed), resumed_cursor_is_deletion:resumed[:next_since]==deleted.id,
  deletion_count:agent.agent_events.where(event_type:'work_unassigned').count, thread_exists:ChannelThread.exists?(thread.id)}

# Actual separate connections: poll, another commit, poll again, then publication.
agent.agent_events.delete_all
thread=make_thread.call('Concurrent commit publication')
since=AgentEvent.maximum(:id).to_i
committed=Queue.new
release=Queue.new
writer=Thread.new do
  ActiveRecord::Base.connection_pool.with_connection do
    Thread.current[:ws11_before_deletion_publication]=-> { committed << true; Timeout.timeout(20) { release.pop } }
    ActiveRecord::Base.transaction do
      thread.destroy!
      make_event.call('github_action_completed')
    end
  ensure
    Thread.current[:ws11_before_deletion_publication]=nil
  end
end
begin
  Timeout.timeout(10) { committed.pop }
  first=poll.call(since)
  interleaved=make_event.call('fizzy_action_completed')
  second=poll.call(first[:next_since])
ensure
  release << true
  writer.value
end
deleted=agent.agent_events.where(event_type:'work_unassigned').sole
third=poll.call(second[:next_since])
result[:concurrent_commits]={first_types:types.call(first), second_types:types.call(second),
  second_cursor_is_interleaved_event:second[:next_since]==interleaved.id, deletion_id_after_interleaved_event:deleted.id>interleaved.id,
  resumed_types:types.call(third), resumed_cursor_is_deletion:third[:next_since]==deleted.id,
  deletion_count:agent.agent_events.where(event_type:'work_unassigned').count, thread_exists:ChannelThread.exists?(thread.id)}
puts JSON.pretty_generate({reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],results:result}.as_json)
