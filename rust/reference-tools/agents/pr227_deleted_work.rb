# Deleted-thread snapshots through the real pinned REST/MCP controllers.
require "json"
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
Rails.logger = ActiveSupport::Logger.new($stderr)
ApplicationJob.queue_adapter = :test
input = JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "reference-tools/agents/pr227-deleted-work-inputs.json")), symbolize_names: true)
raise "wrong reference pin" unless input.fetch(:reference_pin) == "d7c7de92"
secret = "ws11api-fixture-credential"
conn = ActiveRecord::Base.connection
tables = conn.tables.reject { |name| name.start_with?("message_search") || %w[schema_migrations ar_internal_metadata].include?(name) }
tables << "sqlite_sequence"
snapshot = tables.to_h { |table| [table, conn.select_all("SELECT * FROM #{conn.quote_table_name(table)}").to_a] }
restore = -> do
  conn.execute("PRAGMA foreign_keys=OFF")
  conn.execute("DELETE FROM message_search_index")
  conn.transaction do
    tables.each { |table| conn.execute("DELETE FROM #{conn.quote_table_name(table)}") }
    snapshot.each do |table, rows|
      rows.each do |row|
        conn.execute("INSERT INTO #{conn.quote_table_name(table)} (#{row.keys.map { |key| conn.quote_column_name(key) }.join(',')}) VALUES (#{row.values.map { |value| conn.quote(value) }.join(',')})")
      end
    end
  end
  conn.execute("PRAGMA foreign_keys=ON")
end
Rails.application.executor.to_run { ActiveRecord::Base.connection_handler.each_connection_pool.each(&:disable_query_cache!) }
headers = { "Accept" => "application/json", "Content-Type" => "application/json", "User-Agent" => "review227-poll", "Authorization" => "Bearer #{secret}" }
results = []
travel_to Time.utc(2026, 3, 2, 16) do
  input.fetch(:cases).each do |item|
    restore.call
    execution = Rails.application.executor.run!(reset: true)
    begin
      item.fetch(:initial_sql).each { |sql| conn.execute(sql) }
      agent = Agent.find(773018776)
      agent.agent_credentials.delete_all
      agent.agent_credentials.create!(name: "HTTP contract", created_by_id: 127326141, token_digest: AgentCredential.digest(secret), token_last_four: AgentCredential.digest(secret).first(4))
      Rails.cache = ActiveSupport::Cache::MemoryStore.new
      observations = item.fetch(:steps).map do |step|
        queries = []
        cache_hits = 0
        callback = ->(*args) do
          payload = args.last
          next unless payload[:name] != "SCHEMA" && payload[:sql].match?(/\ASELECT\b/i)
          cache_hits += 1 if payload[:cached]
          raise "query cache enabled during #{item.fetch(:key)}" if payload[:cached] || ActiveRecord::Base.connection.query_cache_enabled
          queries << payload[:sql]
        end
        session = ActionDispatch::Integration::Session.new(Rails.application)
        session.host! "campfire.test"
        body = step[:body]&.to_json
        ActiveSupport::Notifications.subscribed(callback, "sql.active_record") do
          session.public_send(step.fetch(:method).downcase, step.fetch(:path), params: body, headers: headers)
        end
        response = session.response
        warn "PR227_POLL_RAILS #{item.fetch(:key)} #{step.fetch(:surface)} selects=#{queries.size} cache_hits=#{cache_hits} status=#{response.status}"
        step.merge(request_body: body, status: response.status, response_body: response.body,
          response_headers: %w[Content-Type Cache-Control Pragma Retry-After Location X-Smartfire-Next-Since].to_h { |name| [name, response.headers[name]] },
          selects: queries.size, queries: queries, cache_hits: cache_hits)
      end
      results << item.except(:steps).merge(observations: observations)
    ensure
      execution.complete!
    end
  end
end
puts JSON.pretty_generate(reference_pin: "d7c7de92", cases: results)
