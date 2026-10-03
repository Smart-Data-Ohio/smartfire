# WS11 assignment-ledger callbacks through WS12's installed work producers.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
Rails.logger = ActiveSupport::Logger.new($stderr)
conn = ActiveRecord::Base.connection
tables = conn.tables.reject { |name| name.start_with?("message_search") || %w[schema_migrations ar_internal_metadata].include?(name) } + ["sqlite_sequence"]
seed = tables.to_h { |table| [table, conn.select_all("SELECT * FROM #{conn.quote_table_name(table)}").to_a] }
restore = -> do
  conn.execute("PRAGMA foreign_keys=OFF")
  conn.execute("DELETE FROM message_search_index")
  conn.transaction do
    tables.each { |table| conn.execute("DELETE FROM #{conn.quote_table_name(table)}") }
    seed.each do |table, rows|
      rows.each do |row|
        conn.execute("INSERT INTO #{conn.quote_table_name(table)} (#{row.keys.map { |key| conn.quote_column_name(key) }.join(',')}) VALUES (#{row.values.map { |value| conn.quote(value) }.join(',')})")
      end
    end
  end
  conn.execute("PRAGMA foreign_keys=ON")
  ApplicationJob.queue_adapter.enqueued_jobs.clear
end
results = {}
travel_to Time.utc(2026, 3, 2, 16) do
  %w[assigned unassigned nonmember human agent status_only outer_commit history_failure ledger_failure].each do |name|
    restore.call
    room = Room.find(486777696)
    bot = User.find(394959859)
    agent = Agent.find(773018776)
    manager = User.find(127326141)
    room.memberships.grant_to(bot)
    AgentGrant.delete_all
    AgentEvent.delete_all
    AgentGrant.create!(agent: agent, room: room, granted_by: manager, capability: "post_messages")
    if %w[nonmember outer_commit].include?(name)
      AgentGrant.create!(agent: agent, room: nil, granted_by: manager, capability: "read_messages")
    end
    other_bot = User.create_bot!(id: 1901600011, name: "Second Owner Bot")
    other = other_bot.create_agent!(id: 1901600012, kind: :workspace, owner: manager)
    room.memberships.grant_to(other_bot)
    AgentGrant.create!(agent: other, room: room, granted_by: manager, capability: "post_messages")
    conn.execute("UPDATE sqlite_sequence SET seq=1901610000 WHERE name='agent_events'")
    conn.execute("UPDATE sqlite_sequence SET seq=1901620000 WHERE name='work_thread_events'")
    thread = ChannelThread.create!(id: 1901600001, room: room, creator: manager, name: "Agent work")
    ThreadMembership.join!(thread, manager)
    thread.update_work!(actor: manager, work_status: "planned")
    error = false
    inside_jobs = nil
    begin
      case name
      when "assigned"
        thread.update_work!(actor: manager, work_owner_id: bot.id)
      when "unassigned", "nonmember", "human", "agent"
        thread.update_work!(actor: manager, work_owner_id: bot.id)
        Membership.where(user: bot, room: room).delete_all if name == "nonmember"
        ApplicationJob.queue_adapter.enqueued_jobs.clear
        target = {"human" => 149087659, "agent" => other_bot.id}.fetch(name, nil)
        thread.update_work!(actor: manager, work_owner_id: target)
      when "status_only"
        thread.update_work!(actor: manager, work_owner_id: 149087659)
        thread.update_work!(actor: manager, work_status: "in_progress")
        thread.update_work!(actor: manager, work_owner_id: manager.id)
        thread.update_work!(actor: manager, work_owner_id: bot.id)
        thread.update_work!(actor: manager, work_status: "blocked")
      when "outer_commit"
        ChannelThread.transaction do
          thread.update_work!(actor: manager, work_owner_id: bot.id)
          inside_jobs = ApplicationJob.queue_adapter.enqueued_jobs.count { |job| job[:job] == Agent::EventWebhookJob }
        end
      else
        table = name == "history_failure" ? "work_thread_events" : "agent_events"
        conn.execute("CREATE TEMP TRIGGER next_reject BEFORE INSERT ON #{table} BEGIN SELECT RAISE(ABORT,'injected named callback failure'); END")
        thread.update_work!(actor: manager, work_owner_id: bot.id)
      end
    rescue ActiveRecord::StatementInvalid
      error = true
    ensure
      conn.execute("DROP TRIGGER IF EXISTS next_reject")
    end
    ledger = AgentEvent.order(:id).map do |event|
      raise "invalid generated chain" unless event.chain_id.match?(/\A[0-9a-f]{8}-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}\z/)
      event.attributes.slice("id", "agent_id", "event_type", "outcome", "message_id", "room_id", "actor_id", "metadata", "hop", "detail", "webhook_status").merge("metadata" => event.metadata)
    end
    history = thread.work_thread_events.order(:id).map do |event|
      event.attributes.slice("id", "event_type", "actor_id", "from_owner_id", "to_owner_id", "from_status", "to_status", "note", "metadata").merge("metadata" => event.metadata)
    end
    jobs = ApplicationJob.queue_adapter.enqueued_jobs.select { |job| job[:job] == Agent::EventWebhookJob }.map { |job| {"event_id" => job[:args].first} }
    thread.reload
    results[name] = {owner: thread.work_owner_id, status: thread.work_status, ledger: ledger, history: history, jobs: jobs, error: error, inside_jobs: inside_jobs}
    raise "fault control failed" if name.end_with?("failure") && (!error || thread.work_owner_id)
  end
end
puts JSON.pretty_generate(reference_pin: "d7c7de92", results: results)
