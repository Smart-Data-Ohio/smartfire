# Complete assertions from channel_thread_handoff_test.rb, including human actors.
require "digest"
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
Rails.logger = ActiveSupport::Logger.new($stderr)
JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "reference-tools/work/handoff-named-source-hashes.json"))).each do |path, hash|
  raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
identify = ->(name) { ActiveRecord::FixtureSet.identify(name) }
keys = %w[history ledger unassign audit webhook current_owner stale_agent stale_human untracked package receiver_access stale_sender_profile]
rows = []
travel_to Time.utc(2026, 3, 2, 16) do
  keys.each do |key|
    david = User.find(identify.call("david"))
    jz = User.find(identify.call("jz"))
    agent = Agent.find(identify.call("bender_agent"))
    board = Rooms::Board.create_for({name: "Launch", creator: david}, users: [david, jz, agent.user])
    %w[read_messages post_messages manage_threads].each { |cap| AgentGrant.create!(agent: agent, room: board, granted_by: david, capability: cap) }
    thread = ChannelThread.create_board_post!(room: board, creator: david, name: "Ship it", work_status: "in_progress", owner_id: david.id)
    sender = david
    summary = "Yours now"
    links = []
    questions = []
    other = nil
    case key
    when "history" then summary = "Halfway there"; links = ["https://example.com/spec"]; questions = ["Which API?"]
    when "ledger" then summary = "Halfway"; links = ["https://example.com/a"]; questions = ["Why?"]
    when "unassign"
      user = User.create!(name: "Clippy", email_address: "clippy@example.com", password: "password123456", role: "bot")
      other = Agent.create!(user: user, owner: david, kind: "workspace")
      board.memberships.grant_to(user)
      %w[read_messages post_messages].each { |cap| AgentGrant.create!(agent: other, room: board, granted_by: david, capability: cap) }
      thread.update_work!(actor: david, work_owner_id: user.id)
    when "current_owner" then thread.update_work!(actor: david, work_owner_id: agent.user_id)
    when "stale_agent"
      thread.update_work!(actor: david, work_owner_id: agent.user_id)
      thread.update_work!(actor: david, work_owner_id: jz.id)
      sender = agent.user
    when "stale_human"
      thread.update_work!(actor: david, work_owner_id: jz.id)
      board.memberships.revoke_from(jz)
      sender = jz
    when "untracked" then thread = ChannelThread.create!(room: Room.find(identify.call("designers")), creator: david, name: "Chat")
    when "package" then summary = "x" * 2001
    when "stale_sender_profile"
      sender = User.find(david.id)
      User.where(id: david.id).update_all(name: "Renamed sender")
    when "receiver_access" then AgentGrant.where(agent: agent, room: board, capability: "post_messages").update_all(revoked_at: Time.current)
    end
    before_event = thread.work_thread_events.maximum(:id).to_i
    before_ledger = AgentEvent.maximum(:id).to_i
    ApplicationJob.queue_adapter.enqueued_jobs.clear
    error = nil
    begin
      thread.hand_off!(sender: sender, receiver_agent: agent, summary: summary, links: links, open_questions: questions)
    rescue ActiveRecord::RecordInvalid => failure
      error = { kind: "invalid", messages: failure.record.errors.full_messages }
    rescue ActiveRecord::RecordNotFound
      error = { kind: "not_found" }
    rescue ChannelThread::WorkUpdateForbidden
      error = { kind: "forbidden" }
    end
    handoff = WorkHandoff.where(channel_thread_id: thread.id).order(:id).last
    history = thread.work_thread_events.where("id > ?", before_event).map do |event|
      {kind: event.event_type, actor: event.actor_id, from_owner_is_previous: event.from_owner_id == (key == "unassign" ? other.user_id : david.id), to_owner: event.to_owner_id,
       handoff_matches: event.metadata["handoff_id"] == handoff&.id, summary: event.metadata["handoff_summary"], links_count: event.metadata["handoff_links_count"], questions_count: event.metadata["handoff_questions_count"]}
    end
    ledger = AgentEvent.where("id > ?", before_ledger).order(:id).map do |event|
      {receiver: event.agent_id == agent.id, kind: event.event_type, outcome: event.outcome, room_matches: event.room_id == board.id, actor: event.actor_id,
       thread_matches: event.metadata["thread_id"] == thread.id, summary: event.metadata.dig("handoff", "summary"), links: event.metadata.dig("handoff", "links"), questions: event.metadata.dig("handoff", "open_questions"), webhook: event.webhook_status}
    end
    audit = AuditLog.where(action: "work.handoff", target_id: thread.id).map { |row| {actor: row.actor_id, target_type: row.target_type, target_matches: row.target_id == thread.id, to_owner: row.details["to_owner"], from_owner: row.details["from_owner"]} }
    jobs = ApplicationJob.queue_adapter.enqueued_jobs.count { |job| job[:job].name == "Agent::EventWebhookJob" }
    rows << {key: key, facts: {error: error, owner: thread.reload.work_owner_id, handoff_count: WorkHandoff.where(channel_thread_id: thread.id).count, handoff_thread_matches: handoff && handoff.channel_thread_id == thread.id, history: history, ledger: ledger, audit: audit, webhook_jobs: jobs}}
  end
end
puts JSON.pretty_generate({rows: rows}.as_json)
warn "Rails human handoff named oracle: #{rows.size} complete assertion sets; 0 masks"
