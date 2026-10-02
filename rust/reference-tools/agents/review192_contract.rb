# PR192 regressions: request-local approval zones and Active Record ID conditions.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
Rails.logger = ActiveSupport::Logger.new($stderr)
secret = "ws11api-fixture-credential"
room = 486777696
thread = 1996000000
cases = []
add = ->(name, method, path, body = nil, setup = {}) { cases << { name: name, method: method, path: path, body: body, setup: setup } }
tool = ->(name, args, label, setup = {}) { add.call(label, :post, "/agents/mcp", { jsonrpc: "2.0", id: 192, method: "tools/call", params: { name: name, arguments: args } }, setup) }
[
  ["America/New_York", "2026-03-03 12:00:00"],
  ["America/New_York", "2026-03-03"],
  ["UTC", "2026-03-03 12:00:00"],
  ["UTC", "2026-03-03"],
  ["Asia/Kolkata", "2026-03-03 12:00:00"],
  ["Eastern Time (US & Canada)", "2026-03-03 12:00:00"],
  ["America/New_York", "2026-03-08 02:30:00"],
  ["America/New_York", "2026-11-01 01:30:00"],
  ["Australia/Lord_Howe", "2026-10-04 02:15:00"],
  ["America/New_York", "2026-03-03T12:00:00+03:00"],
  ["America/New_York", "not a deadline"],
  ["America/New_York", "2026-03-03 12:00:00", { expires_in: 7200 }]
].each_with_index do |(zone, expiry, extra), i|
  args = { action: "deploy", summary: "Review deadline", expires_at: expiry }.merge(extra || {})
  setup = { zone: zone, approval: true }
  add.call("approval_rest_#{i}", :post, "/agents/approvals", args, setup)
  tool.call("request_approval", args, "approval_mcp_#{i}", setup)
end
{ message_id: 935961918, thread_id: thread }.each do |key, id|
  [[id], [0, id], [[id]], [], [0], { id: id }].each_with_index do |value, i|
    add.call("context_rest_#{key}_#{i}", :get, "/agents/context?#{Rack::Utils.build_nested_query(key => value, limit: 2)}")
    tool.call("get_context", { key => value, limit: 2 }, "context_mcp_#{key}_#{i}")
  end
end
tool.call("get_context", { message_id: [1996002000], thread_id: [thread] }, "context_mcp_both_arrays")
tool.call("get_context", { message_id: [1996002000], thread_id: thread }, "context_mcp_message_array_with_thread")
tool.call("read_messages", { room_id: [room], before: [1996001002], after: [1996001000], limit: 2 }, "history_root_array_ids")
tool.call("read_messages", { thread_id: [thread], before: [1996002002], after: [1996002000], limit: 2 }, "history_thread_array_ids")
[[thread], [0, thread], [[thread]], [], [0], { id: thread }].each_with_index do |value, i|
  tool.call("get_work", { work_id: value }, "work_array_ids_#{i}")
end
tool.call("list_board_posts", { room_id: [room] }, "board_array_id", { board: true })
travel_to Time.utc(2026, 3, 2, 16) do
  results = cases.map do |item|
    result = nil
    execution = Rails.application.executor.run!(reset: true)
    begin
      ActiveRecord::Base.transaction do
        agent = Agent.find(773018776)
        agent.agent_events.delete_all; agent.agent_grants.delete_all; agent.agent_credentials.delete_all
        agent.update_columns(status: "idle", owner_id: 127326141)
        agent.agent_credentials.create!(name: "HTTP contract", created_by_id: 127326141, token_digest: AgentCredential.digest(secret), token_last_four: AgentCredential.digest(secret).first(4))
        conn = ActiveRecord::Base.connection
        conn.execute("INSERT INTO channel_threads(id,name,room_id,creator_id,work_owner_id,work_status,last_activity_at,created_at,updated_at,messages_count) VALUES(#{thread},'Query probe',#{room},127326141,394959859,'in_progress','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00',50)")
        50.times do |n|
          [[1996001000 + n, "NULL"], [1996002000 + n, thread.to_s]].each do |id, tid|
            conn.execute("INSERT INTO messages(id,room_id,creator_id,thread_id,client_message_id,markdown_source,created_at,updated_at) VALUES(#{id},#{room},127326141,#{tid},'pr192-read-#{id}','Query probe','2026-03-02 16:00:00','2026-03-02 16:00:00')")
            conn.execute("INSERT INTO action_text_rich_texts(name,record_type,record_id,body,created_at,updated_at) VALUES('body','Message',#{id},'<p>Query probe</p>','2026-03-02 16:00:00','2026-03-02 16:00:00')")
          end
        end
        if item[:setup][:approval]
          User.find(agent.user_id).update_columns(time_zone: item[:setup][:zone])
          AgentGrant.create!(agent: agent, capability: "external_action", granted_by_id: 127326141)
        end
        conn.execute("UPDATE rooms SET type='Rooms::Board' WHERE id=#{room}") if item[:setup][:board]
        Rails.cache = ActiveSupport::Cache::MemoryStore.new
        session = ActionDispatch::Integration::Session.new(Rails.application)
        session.host! "campfire.test"
        session.public_send(item[:method], item[:path], params: item[:body]&.to_json, headers: { "Accept" => "application/json", "Content-Type" => "application/json", "Authorization" => ["Bearer", secret].join(" ") })
        response = session.response
        result = item.merge(method: item[:method].to_s.upcase, body: item[:body]&.to_json, status: response.status, response: response.body, headers: response.headers.slice("cache-control", "content-type"))
        if item[:setup][:approval]
          result[:stored_deadline] = AgentApproval.where(agent_id: agent.id).order(id: :desc).first&.expires_at&.utc&.iso8601(6)
        end
        raise ActiveRecord::Rollback
      end
    ensure
      execution.complete!
    end
    result
  end
  puts JSON.pretty_generate({ pin: "d7c7de92", cases: results })
end
