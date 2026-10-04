# Observe actual commit callbacks and rendered Action Cable broadcasts on the current Rails reference pin.
require 'active_support/testing/time_helpers'
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026, 3, 2, 16)
agent = Agent.find(773018776)
room = Room.find(486777696)
human = User.find(127326141)
AgentGrant.delete_all
room.memberships.grant_to([agent.user])
results = {}
[false, true].each do |webhook|
  if webhook
    Webhook.find_or_create_by!(user: agent.user) { |w| w.url = 'https://bots.example.test/hook' }
  else
    Webhook.where(user: agent.user).delete_all
  end
  [false, true].each do |reject|
    key = "webhook_#{webhook}_reject_#{reject}"
    parent = Message.create!(room: room, creator: human, client_message_id: "ws11-deletion-indicator-#{key}", markdown_source: 'Parent')
    thread = ChannelThread.create!(room: room, creator: human, parent_message: parent, name: 'Indicator deletion', work_status: 'planned')
    thread.update_columns(work_owner_id: agent.user_id)
    thread.post_message!(creator: human, attributes: { markdown_source: 'Reply' })
    agent.agent_events.delete_all
    frames = []
    observer = ActiveSupport::Notifications.subscribe('broadcast.action_cable') do |*args|
      event = ActiveSupport::Notifications::Event.new(*args)
      frames << event.payload[:message] if event.payload[:broadcasting] == Turbo::StreamsChannel.send(:stream_name_from, [room, :messages])
    end
    if reject
      ActiveRecord::Base.connection.execute("CREATE TEMP TRIGGER ws11_reject_indicator_ledger BEFORE INSERT ON agent_events WHEN NEW.event_type='work_unassigned' AND json_extract(NEW.metadata,'$.thread_id')=#{thread.id} BEGIN SELECT RAISE(ABORT,'WS11 rejected indicator ledger'); END")
    end
    error = nil
    begin
      thread.destroy!
    rescue ActiveRecord::StatementInvalid
      error = 'statement_invalid'
    ensure
      ActiveSupport::Notifications.unsubscribe(observer)
      ActiveRecord::Base.connection.execute('DROP TRIGGER ws11_reject_indicator_ledger') if reject
    end
    results[key] = {
      error: error,
      thread_exists: ChannelThread.exists?(thread.id),
      parent_exists: Message.exists?(parent.id),
      replies_remaining: Message.where(thread_id: thread.id).count,
      deletion_events: agent.agent_events.where(event_type: 'work_unassigned').count,
      indicator_frames: frames.grep(String).select { |frame| frame.include?("thread_indicator_message_#{parent.client_message_id}") }
    }
  end
end
puts JSON.pretty_generate({ reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], results: results }.as_json)
