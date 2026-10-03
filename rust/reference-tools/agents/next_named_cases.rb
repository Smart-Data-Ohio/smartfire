# Individually named WS11 follow-ups, through real models and committed writes.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
Rails.logger = ActiveSupport::Logger.new($stderr)
results = {}
travel_to Time.utc(2026, 3, 2, 16) do
  bot = User.find(394959859)
  room = Room.find(486777696)
  agent = Agent.find(773018776)
  human = User.find(127326141)
  room.memberships.grant_to(bot)
  AgentGrant.delete_all
  AgentEvent.delete_all

  reply_bot = User.create_bot!(id: 1901500001, name: "Reply model bot")
  room.memberships.grant_to(reply_bot)
  token = reply_bot.reply_token_for(room)
  authenticate = ->(value, target = room.id) { User.authenticate_bot_reply_token(value, room_id: target)&.id }
  results[:reply_room] = {
    token: token, bot_id: reply_bot.id, room_id: room.id,
    valid: authenticate.call(token), wrong_room: authenticate.call(token, 201306877),
    tampered: authenticate.call("#{token}x"), malformed: authenticate.call("bogus"),
    blank: authenticate.call(""), key_auth: User.authenticate_bot(token)&.id
  }
  raise "reply-token positive control failed" unless results[:reply_room][:valid] == reply_bot.id
  short = reply_bot.reply_token_for(room, expires_in: 1.minute)
  initial = authenticate.call(short)
  travel_to Time.utc(2026, 3, 2, 16, 2)
  results[:reply_expiry] = {token: short, before: initial, after: authenticate.call(short)}
  raise "reply-token expiry control failed" unless initial == reply_bot.id && results[:reply_expiry][:after].nil?
  travel_to Time.utc(2026, 3, 2, 16)
  Membership.where(user: reply_bot, room: room).delete_all
  absent = authenticate.call(token)
  room.memberships.grant_to(reply_bot)
  restored = authenticate.call(token)
  reply_bot.deactivate
  results[:reply_membership] = {absent: absent, restored: restored, deactivated: authenticate.call(token)}

  receiver_bot = User.create_bot!(name: "Self Hop Bot B")
  receiver = receiver_bot.create_agent!(kind: :workspace, owner: human)
  room.memberships.grant_to(receiver_bot)
  board = Rooms::Board.create_for({name: "Self Hop Board", creator: human}, users: [human, bot])
  message = room.messages.create!(creator: human, markdown_source: "Hey @[#{bot.name}]", client_message_id: "next-self-hop-trigger")
  event = agent.agent_events.deliverable.last
  Agent::DeliveryJob.perform_now(event.id)
  initial_hop = event.reload.hop
  2.times do |i|
    ChannelThread.create_board_post!(room: board, creator: bot, name: "Self post #{i}", work_status: "in_progress", owner_id: bot.id)
  end
  room.messages.create!(creator: bot, markdown_source: "Hey @[#{receiver_bot.name}] help", client_message_id: "next-self-hop-handoff")
  received = receiver.agent_events.deliverable.last
  results[:self_hop] = {
    initial_hop: initial_hop, self_assignments: agent.agent_events.where(event_type: "work_assigned").count,
    receiver_hop: received&.hop, receiver_outcome: received&.outcome,
    suppressed: receiver.agent_events.where(event_type: "delivery_suppressed_hop_limit").count
  }

  AgentGrant.create!(agent: agent, room: board, granted_by: human, capability: "post_messages")
  owned = ChannelThread.create_board_post!(room: board, creator: human, name: "Ship it", work_status: "in_progress", owner_id: bot.id)
  history = owned.work_thread_events.count
  agent.kill_switch!
  results[:kill_owned] = {
    owner: owned.reload.work_owner_id, status: owned.work_status,
    active: owned.work_owner_active?, history_before: history, history_after: owned.work_thread_events.count,
    suspended: agent.reload.suspended?
  }

  # Observe the actual rate SELECT under the agent lock. Rust's counterpart
  # challenges the serialization boundary with two independent SQLite writers.
  agent.update_columns(suspended_at: nil)
  AgentGrant.delete_all
  AgentEvent.delete_all
  19.times { agent.agent_events.create!(event_type: "mention", room: room, actor: human, outcome: "delivered") }
  ApplicationJob.queue_adapter.enqueued_jobs.clear
  depths = []
  subscriber = ->(*args) do
    sql = args.last[:sql]
    depths << ActiveRecord::Base.connection.open_transactions if sql.match?(/SELECT COUNT.*agent_events/i)
  end
  ActiveSupport::Notifications.subscribed(subscriber, "sql.active_record") do
    2.times { |i| room.messages.create!(creator: human, markdown_source: "Hey @[#{bot.name}]", client_message_id: "next-rate-lock-#{i}") }
  end
  raise "rate SELECT must run twice inside the agent lock" unless depths.size == 2 && depths.all?(&:positive?)
  results[:rate_lock] = {
    checks: depths.size, locked: depths.all?(&:positive?),
    eligible: agent.agent_events.where(event_type: "mention", outcome: %w[pending delivered acknowledged]).count,
    suppressed: agent.agent_events.where(event_type: "delivery_suppressed_rate_limit").count,
    jobs: ApplicationJob.queue_adapter.enqueued_jobs.count { |job| job[:job] == Agent::DeliveryJob }
  }
end
puts JSON.pretty_generate(reference_pin: "d7c7de92", results: results)
