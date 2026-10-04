# The sixteen WS12 declarations in channel_thread_agent_assignment_test.rb.
# Every field is read from the actual persisted model/recorder path, not a response mask.
require "digest"
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
Rails.logger = ActiveSupport::Logger.new($stderr)
source_files = %w[test/models/channel_thread_agent_assignment_test.rb app/models/channel_thread.rb app/models/work_thread_event.rb app/services/activity_items/recorder.rb]
hashes = JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "reference-tools/work/agent-named-source-hashes.json")))
hashes.each { |path, hash| raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash }
identify = ->(name) { ActiveRecord::FixtureSet.identify(name) }
names = [
  ["active", "an active member agent with post_messages is an eligible work owner"],
  ["legacy", "a legacy agent keeps post eligibility through the fallback"],
  ["suspended", "a suspended agent is rejected with a validation error"],
  ["nonmember", "a non-member agent is rejected with a validation error"],
  ["no_post", "an agent without post_messages is rejected with a validation error"],
  ["outside_human", "a human outside the parent room keeps the human eligibility error"],
  ["plain_bot", "a bot without an agent row is rejected with a validation error"],
  ["unavailable", "suspending the agent or revoking its membership reads as an unavailable owner"],
  ["status_note", "an agent status update records a work event with the note"],
  ["status_inbox", "an agent status update reaches the inbox through the human path"],
  ["tags", "an agent tags update replaces the set without touching the status"],
  ["invalid_fields", "an agent work update validates tags and run_url"],
  ["result", "an agent result update records the event with the agent as actor"],
  ["unowned_result", "an agent cannot write the result of work it does not own"],
  ["unowned_status", "an agent cannot move work it does not own"],
  ["invalid_status_note", "an agent status update rejects unknown statuses and long notes"]
]
rows = []
travel_to Time.utc(2026, 3, 2, 16) do
  names.each_with_index do |(key, name), index|
    # Each case uses independent association/policy state; callbacks run after real commits.
    AgentGrant.where(agent_id: identify.call("bender_agent")).delete_all
    agent = Agent.find(identify.call("bender_agent"))
    agent.update_columns(suspended_at: nil)
    bot = agent.user
    bot.update_columns(status: User.statuses.fetch("active"))
    room = Room.find(identify.call("watercooler"))
    manager = User.find(identify.call("david"))
    room.memberships.grant_to(bot)
    grant = ->(cap) { AgentGrant.create!(agent: agent, room: room, granted_by: manager, capability: cap) }
    errors = []
    transitions = []
    other = nil
    if key == "status_inbox"
      room = Room.find(identify.call("designers"))
      manager = User.find(identify.call("jz"))
      bot = User.create_bot!(id: 901830001, name: "Inbox Worker Bot")
      agent = Agent.create!(id: 901830002, user: bot, kind: :workspace, owner_id: identify.call("david"))
      room.memberships.grant_to(bot)
      grant = ->(cap) { AgentGrant.create!(agent: agent, room: room, granted_by: User.find(identify.call("david")), capability: cap) }
    end
    thread = ChannelThread.create!(id: 901831000 + index, room: room, creator: manager, name: key == "status_inbox" ? "Inbox agent work" : "Agent work")
    ThreadMembership.join!(thread, manager)
    ThreadMembership.join!(thread, User.find(identify.call("david"))).update!(involvement: "everything") if key == "status_inbox"
    thread.update_work!(actor: manager, work_status: "planned")
    attempt = ->(&operation) do
      begin
        operation.call
        errors << nil
      rescue ActiveRecord::RecordInvalid => error
        errors << { kind: "invalid", messages: error.record.errors.full_messages }
      rescue ActiveRecord::RecordNotFound
        errors << { kind: "not_found" }
      end
    end
    if %w[active legacy suspended nonmember no_post outside_human plain_bot].include?(key)
      grant.call(key == "no_post" ? "read_messages" : "post_messages") unless key == "legacy"
      agent.update_columns(suspended_at: Time.current) if key == "suspended"
      room.memberships.find_by!(user: bot).destroy! if key == "nonmember"
      owner = if key == "outside_human"
        User.find(identify.call("kevin"))
      elsif key == "plain_bot"
        user = User.create_bot!(id: 901830003, name: "No Agent Bot")
        room.memberships.grant_to(user)
        user
      else
        bot
      end
      attempt.call { thread.update_work!(actor: manager, work_owner_id: owner.id) }
    elsif key == "unavailable"
      grant.call("post_messages")
      thread.update_work!(actor: manager, work_owner_id: bot.id)
      other_bot = User.create_bot!(id: 901830004, name: "Suspended Owner Bot")
      other_agent = Agent.create!(id: 901830005, user: other_bot, kind: :workspace, owner: manager)
      room.memberships.grant_to(other_bot)
      AgentGrant.create!(agent: other_agent, room: room, granted_by: manager, capability: "post_messages")
      other = ChannelThread.create!(id: 901832000, room: room, creator: manager, name: "Suspended work")
      ThreadMembership.join!(other, manager)
      other.update_work!(actor: manager, work_status: "planned", work_owner_id: other_bot.id)
      transitions << [thread.reload.work_owner_active?, other.reload.work_owner_active?]
      room.memberships.find_by!(user: bot).destroy!
      other_agent.update_columns(suspended_at: Time.current)
      transitions << [thread.reload.work_owner_active?, other.reload.work_owner_active?]
    else
      grant.call("post_messages")
      thread.update_work!(actor: manager, work_owner_id: %w[unowned_result unowned_status].include?(key) ? identify.call("jason") : bot.id)
      case key
      when "status_note" then attempt.call { thread.update_work_by_agent!(agent: agent, work_status: "in_progress", note: "Digging in") }
      when "status_inbox" then attempt.call { thread.update_work_by_agent!(agent: agent, work_status: "in_progress", note: "On it") }
      when "tags" then attempt.call { thread.update_work_by_agent!(agent: agent, tags: "API, launch") }
      when "invalid_fields"
        attempt.call { thread.update_work_by_agent!(agent: agent, tags: "one, two, three, four, five, six") }
        attempt.call { ChannelThread.find(thread.id).update_work_by_agent!(agent: agent, run_url: "http://example.com/runs/1") }
        attempt.call { ChannelThread.find(thread.id).update_work_by_agent!(agent: agent) }
      when "result" then attempt.call { thread.update_result_by_agent!(agent: agent, markdown: "## Agent outcome") }
      when "unowned_result" then attempt.call { thread.update_result_by_agent!(agent: agent, markdown: "Hijacked") }
      when "unowned_status" then attempt.call { thread.update_work_by_agent!(agent: agent, work_status: "in_progress") }
      when "invalid_status_note"
        attempt.call { thread.update_work_by_agent!(agent: agent, work_status: "shipped") }
        attempt.call { ChannelThread.find(thread.id).update_work_by_agent!(agent: agent, work_status: "in_progress", note: "x" * 501) }
      end
    end
    thread.reload
    events = thread.work_thread_events.order(:id)
    latest = events.last
    inbox = ActivityItem.where(source_type: "WorkThreadEvent", source_id: events.select(:id)).order(:user_id, :id)
      .map { |item| { user: item.user_id, event_type: item.event_type, latest: item.source_id == latest.id, unread: item.unread? } }
    history = events.map { |event| { kind: event.event_type, actor: event.actor_id, from_owner: event.from_owner_id, to_owner: event.to_owner_id, from_status: event.from_status, to_status: event.to_status, note: event.note, metadata: event.metadata } }
    facts = { errors: errors, owner: thread.work_owner_id, status: thread.work_status, tags: thread.tag_names, result: thread.result_markdown, result_actor: thread.result_updated_by_id, run_url: thread.run_url, available: thread.work_owner_active?, transitions: transitions, other_owner: other&.reload&.work_owner_id, history: history, inbox: inbox }
    rows << { key: key, rails_test: name, facts: facts }
  end
end
puts JSON.pretty_generate({ reference: ENV.fetch("PARITY_REFERENCE_SHA"), rows: rows }.as_json)
warn "Rails WS12 agent named oracle: #{rows.size} complete owner/mutation assertion sets; 0 masks"
