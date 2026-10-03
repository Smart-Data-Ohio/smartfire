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
%w[thread before after react context_message context_thread rest_context_message rest_context_thread].each do |label|
  [5, 50].each do |size|
    target = label.include?("thread") ? thread : label == "before" ? 1996002049 : 1996002000
    ids = (1..size).map { |n| -n } + [target]
    if label.start_with?("rest_")
      key = label.end_with?("thread") ? :thread_id : :message_id
      add.call("#{label}_#{size}", :get, "/agents/context?#{Rack::Utils.build_nested_query(key => ids, limit: 5)}")
    else
      name, args = case label
      when "thread" then ["read_messages", { thread_id: ids, limit: 5 }]
      when "before", "after" then ["read_messages", { thread_id: thread, label => ids, limit: 5 }]
      when "react" then ["react", { message_id: ids, content: "👍" }]
      when "context_message" then ["get_context", { message_id: ids, limit: 5 }]
      when "context_thread" then ["get_context", { thread_id: ids, limit: 5 }]
      end
      # Use the same JSON-RPC id as the committed Rust request.
      add.call("#{label}_#{size}", :post, "/agents/mcp", { jsonrpc: "2.0", id: 196, method: "tools/call", params: { name: name, arguments: args } })
    end
  end
end
# Multiple valid candidates select in primary-key order, not input order.
[["before", [1996002049, 1996002040]], ["after", [1996002004, 1996002000]], ["before", [1996001002, 1996002049]], ["after", [935961918, 1996002000]]].each_with_index do |(key, ids), i|
  add.call("scoped_order_#{i}", :post, "/agents/mcp", { jsonrpc:"2.0", id:196, method:"tools/call", params:{name:"read_messages", arguments:{thread_id:thread, key=>ids, limit:5}} })
end
[
  ["global_thread_before_membership", {thread_id:[1996000000,1986000000],limit:{bad:true}}, {}],
  ["room_membership_scoped_in", {room_id:[201306877,486777696],limit:5}, {}],
  ["thread_revoked_before_invalid_limit", {thread_id:[-1,1996000000],limit:{bad:true}}, {revoked:true}],
  ["inactive_agent_before_missing_thread", {thread_id:[-1],limit:{bad:true}}, {inactive:true}]
].each do |label,args,setup|
  add.call(label,:post,"/agents/mcp",{jsonrpc:"2.0",id:196,method:"tools/call",params:{name:"read_messages",arguments:args}},setup)
end
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
        conn.execute("INSERT INTO channel_threads(id,name,room_id,creator_id,last_activity_at,created_at,updated_at) VALUES(1986000000,'Hidden',201306877,127326141,'2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')")
        AgentGrant.create!(agent:agent,capability:"read_messages",granted_by_id:127326141,revoked_at:Time.current) if item[:setup][:revoked]
        agent.update_columns(status:"suspended",suspended_at:Time.current) if item[:setup][:inactive]
        50.times do |n|
          [[1996001000 + n, "NULL"], [1996002000 + n, thread.to_s]].each do |id, tid|
            conn.execute("INSERT INTO messages(id,room_id,creator_id,thread_id,client_message_id,markdown_source,created_at,updated_at) VALUES(#{id},#{room},127326141,#{tid},'pr192-read-#{id}','Query probe','2026-03-02 16:00:00','2026-03-02 16:00:00')")
            conn.execute("INSERT INTO action_text_rich_texts(name,record_type,record_id,body,created_at,updated_at) VALUES('body','Message',#{id},'<p>Query probe</p>','2026-03-02 16:00:00','2026-03-02 16:00:00')")
          end
        end
        Rails.cache = ActiveSupport::Cache::MemoryStore.new
        session = ActionDispatch::Integration::Session.new(Rails.application)
        session.host! "campfire.test"
        request_headers = { "Accept" => "application/json", "Content-Type" => "application/json", "Authorization" => ["Bearer", secret].join(" ") }
        send_request = -> { session.public_send(item[:method], item[:path], params: item[:body]&.to_json, headers: request_headers) }
        send_request.call
        selects = []
        listener = ->(_name, _start, _finish, _id, payload) { selects << payload[:sql] if payload[:sql].lstrip.start_with?("SELECT") && payload[:name] != "SCHEMA" }
        ActiveRecord::Base.uncached { ActiveSupport::Notifications.subscribed(listener, "sql.active_record") { send_request.call } }
        response = session.response
        result = item.merge(method: item[:method].to_s.upcase, body: item[:body]&.to_json, status: response.status, response: response.body, headers: response.headers.slice("cache-control", "content-type"), selects: selects.length)
        raise ActiveRecord::Rollback
      end
    ensure
      execution.complete!
    end
    result
  end
  puts JSON.pretty_generate({ pin: "d7c7de92", cases: results })
end
