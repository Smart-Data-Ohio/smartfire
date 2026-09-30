# Actual Rails REST/MCP requests. Case setup is replayed against the seeded Rust app.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
Rails.logger = ActiveSupport::Logger.new($stderr)
SECRET = "ws11api-fixture-credential"
CASES = []
def contract(name, method, path, body = nil, setup = {})
  setup = { grant: "external_action" }.merge(setup) if path.include?("approvals") || (body.is_a?(Hash) && %w[request_approval get_approval].include?(body.dig(:params, :name)))
  CASES << { name: name, method: method, path: path, body: body, setup: setup }
end
def tool(name, arguments = {}, setup = {}, label = name)
  contract("mcp_#{label}", :post, "/agents/mcp", { jsonrpc: "2.0", id: 7, method: "tools/call", params: { name: name, arguments: arguments } }, setup)
end
contract("approvals_list", :get, "/agents/approvals")
contract("approvals_show", :get, "/agents/approvals/900100001", nil, { approval: "pending" })
contract("approvals_expired", :get, "/agents/approvals/900100001", nil, { approval: "expired" })
contract("approvals_cancel", :delete, "/agents/approvals/900100001", nil, { approval: "pending" })
contract("approvals_cancel_decided", :delete, "/agents/approvals/900100001", nil, { approval: "approved" })
contract("approvals_missing", :get, "/agents/approvals/0")
contract("approvals_nested", :post, "/agents/approvals", { approval: { action: "deploy", summary: "Ship", payload: { x: [1, true] }, room_id: 486777696, expires_in_seconds: 3600 } })
contract("approvals_top", :post, "/agents/approvals", { action: "deploy", summary: "Ship", expires_at: "2026-03-03T16:00:00Z" })
contract("approvals_invalid", :post, "/agents/approvals", { action: "Bad Action!", summary: "" })
contract("approvals_invalid_expiry", :post, "/agents/approvals", { action: "deploy", summary: "Ship", expires_in: "abc" })
contract("approvals_github", :post, "/agents/approvals", { action: "github.comment", summary: "Ship" })
contract("approvals_fizzy", :post, "/agents/approvals", { action: "fizzy.create", summary: "Ship" })
contract("approvals_nonmember", :post, "/agents/approvals", { action: "deploy", summary: "Ship", room_id: 201306877 })
contract("approvals_denied", :post, "/agents/approvals", { action: "deploy", summary: "Ship" }, { grant: "read_messages" })
contract("approvals_replay", :post, "/agents/approvals", { action: "github.comment", summary: "Ignored", external_id: "surface-replay", expires_in: "abc" }, { approval: "pending", cap: 0 })
contract("approvals_budget", :post, "/agents/approvals", { action: "deploy", summary: "Ship" }, { cap: 0 })
contract("approvals_rate", :get, "/agents/approvals", nil, { repeat: 120 })
contract("approvals_create_rate", :post, "/agents/approvals", {}, { repeat: 60 })
tool("request_approval", { action: "deploy", summary: "Ship", payload: { x: 1 } })
tool("request_approval", {}, {}, "request_approval_invalid")
tool("get_approval", { approval_id: 900100001 }, { approval: "pending" })
tool("get_approval", { approval_id: 0 }, {}, "get_approval_missing")
tool("request_approval", { action: "deploy", summary: "Ship" }, { grant: "read_messages" }, "request_approval_denied")
tool("request_approval", { action: "deploy", summary: "Ship" }, { cap: 0 }, "request_approval_budget")

contract("context_required", :get, "/agents/context")
contract("context_missing", :get, "/agents/context?message_id=0")
contract("context_grant", :get, "/agents/context?message_id=935961918", nil, { grant: "post_messages" })
contract("context_mismatch", :get, "/agents/context?message_id=935961918&thread_id=9000")
contract("dm_missing", :post, "/agents/dms", { user_id: 0, body: "hi" })
contract("dm_bot", :post, "/agents/dms", { user_id: 394959859, body: "hi" })
contract("dm_forbidden", :post, "/agents/dms", { user_id: 149087659, body: "hi" })
contract("dm_grant", :post, "/agents/dms", { user_id: 127326141, body: "hi" }, { grant: "read_messages" })
contract("message_nonmember", :post, "/rooms/201306877/agents/messages", { message: { body: "hi" } })
contract("message_grant", :post, "/rooms/486777696/agents/messages", { message: { body: "hi" } }, { grant: "read_messages" })
contract("message_budget", :post, "/rooms/486777696/agents/messages", { message: { body: "hi" } }, { message_cap: 0 })
contract("stream_missing", :patch, "/agents/streaming_messages/0", { append: "hi" })
contract("stream_not_streaming", :patch, "/agents/streaming_messages/935961918", { append: "hi" }, { own_message: true })
contract("stream_grant", :patch, "/agents/streaming_messages/935961918", { append: "hi" }, { own_message: true, grant: "read_messages" })
contract("stream_finalize_missing", :post, "/agents/streaming_messages/0/finalize", {})
contract("stream_start_nonmember", :post, "/rooms/201306877/agents/streaming_messages", { message: { markdown_source: "hi" } })
contract("stream_start_grant", :post, "/rooms/486777696/agents/streaming_messages", { message: { markdown_source: "hi" } }, { grant: "read_messages" })
contract("pin_missing", :post, "/agents/messages/0/pin", {})
contract("pin_grant", :post, "/agents/messages/935961918/pin", {}, { grant: "read_messages" })
contract("pin_delete_missing", :delete, "/agents/messages/0/pin")
contract("poll_options", :post, "/rooms/486777696/agents/polls", { question: "Choose", options: [] })
contract("poll_question", :post, "/rooms/486777696/agents/polls", { question: " ", options: ["a", "b"] })
contract("poll_nonmember", :post, "/rooms/201306877/agents/polls", {})
contract("poll_grant", :post, "/rooms/486777696/agents/polls", {}, { grant: "read_messages" })
contract("poll_missing", :get, "/rooms/486777696/agents/polls/0")
contract("posts_not_board", :get, "/rooms/486777696/agents/posts")
contract("posts_grant", :get, "/rooms/486777696/agents/posts", nil, { grant: "post_messages" })
contract("posts_create_grant", :post, "/rooms/486777696/agents/posts", { title: "Hi" })
contract("posts_create_not_board", :post, "/rooms/486777696/agents/posts", { title: "Hi" }, { grant: ["post_messages", "manage_threads"] })
contract("work_missing", :get, "/agents/work/0")
contract("work_update_missing", :patch, "/agents/work/0", {})
contract("work_result_missing", :put, "/agents/work/0/result", {})
contract("work_handoff_missing", :post, "/agents/work/0/handoff", {})
contract("work_manage_grant", :patch, "/agents/work/900200001", {}, { owned_work: true, grant: "read_messages" })
contract("work_result_required", :put, "/agents/work/900200001/result", {}, { owned_work: true, grant: ["read_messages", "manage_threads"] })
tool("get_context", {}, {}, "context_required")
tool("get_context", { message_id: 0 }, {}, "context_missing")
tool("read_messages", {}, {}, "reading_required")
tool("read_messages", { room_id: 486777696, thread_id: 1 }, {}, "reading_two_conversations")
tool("read_messages", { room_id: 201306877 }, {}, "reading_nonmember")
tool("read_messages", { room_id: 486777696 }, { grant: "post_messages" }, "reading_grant")
tool("read_messages", { room_id: 486777696, before: 0 }, {}, "reading_cursor")
tool("post_message", { room_id: 486777696 }, {}, "message_required_body")
tool("post_message", { room_id: 201306877, body: "hi" }, {}, "message_nonmember")
tool("post_message", { room_id: 486777696, body: "hi" }, { message_cap: 0 }, "message_budget")
tool("open_dm", { user_id: 0, body: "hi" }, {}, "dm_missing")
tool("open_dm", { user_id: 394959859, body: "hi" }, {}, "dm_bot")
tool("open_dm", { user_id: 149087659, body: "hi" }, {}, "dm_forbidden")
tool("react", { message_id: 0, content: "eyes" }, {}, "reaction_missing")
tool("pin_message", { message_id: 0 }, {}, "pin_missing")
tool("unpin_message", { message_id: 0 }, {}, "pin_delete_missing")
tool("create_poll", { room_id: 486777696, question: "Q", options: "a" }, {}, "poll_array")
tool("create_poll", { room_id: 486777696, question: "Q", options: ["a"] }, {}, "poll_options")
tool("get_poll", { room_id: 486777696, poll_id: 0 }, {}, "poll_missing")
tool("start_stream", { room_id: 201306877 }, {}, "stream_start_missing")
tool("append_stream", { message_id: 935961918, append: "hi" }, { own_message: true }, "stream_not_streaming")
tool("finalize_stream", { message_id: 0 }, {}, "stream_finalize_missing")
tool("list_board_posts", { room_id: 486777696 }, {}, "posts_not_board")
tool("create_board_post", { room_id: 486777696 }, {}, "posts_manage_grant")
tool("update_board_post", { post_id: 0 }, {}, "work_post_missing")
tool("update_work", { work_id: 0 }, {}, "work_missing")
tool("set_result", { post_id: 900200001 }, { owned_work: true, grant: ["read_messages", "manage_threads"] }, "work_result_required")
tool("handoff_work", { work_id: 0, receiver_agent_id: 1, summary: "handoff" }, {}, "work_handoff_missing")
# Integration clients are not invoked by any of these local boundary failures.
contract("fizzy_boards_grant", :get, "/agents/fizzy/boards", nil, { grant: "read_messages" })
contract("fizzy_board_grant", :get, "/agents/fizzy/boards/abc", nil, { grant: "read_messages" })
contract("fizzy_search_grant", :get, "/agents/fizzy/cards/search?q=hi", nil, { grant: "read_messages" })
contract("fizzy_card_grant", :get, "/agents/fizzy/cards/acct/1", nil, { grant: "read_messages" })
contract("fizzy_action_grant", :post, "/agents/fizzy/card_actions", { kind: "close", number: 1 }, { grant: "fizzy" })
contract("fizzy_no_account", :get, "/agents/fizzy/boards", nil, { grant: "fizzy" })
contract("fizzy_no_owner", :get, "/agents/fizzy/boards", nil, { grant: "fizzy", ownerless: true })
contract("fizzy_bad_account", :get, "/agents/fizzy/boards?account_id=bad%2Fid", nil, { grant: "fizzy", fizzy_account: true })
contract("fizzy_bad_board", :get, "/agents/fizzy/boards/bad%20id", nil, { grant: "fizzy", fizzy_account: true })
contract("fizzy_bad_card_number", :get, "/agents/fizzy/cards/acct/abc", nil, { grant: "fizzy", fizzy_account: true })
contract("fizzy_search_blank", :get, "/agents/fizzy/cards/search?q=%20", nil, { grant: "fizzy", fizzy_account: true })
contract("fizzy_action_no_account", :post, "/agents/fizzy/card_actions", {}, { grant: "external_action" })
contract("fizzy_boards_rate", :get, "/agents/fizzy/boards", nil, { grant: "read_messages", repeat: 120 })
contract("fizzy_action_rate", :post, "/agents/fizzy/card_actions", {}, { grant: "fizzy", repeat: 60 })
contract("github_nonmember", :post, "/rooms/201306877/agents/github/pull_request_actions", {})
contract("github_missing_pr", :post, "/rooms/486777696/agents/github/pull_request_actions", { pull_request_id: 0 })
contract("github_grant", :post, "/rooms/486777696/agents/github/pull_request_actions", { pull_request_id: 900400001 }, { github_pr: true, grant: "read_messages" })
contract("github_no_account", :post, "/rooms/486777696/agents/github/pull_request_actions", { pull_request_id: 900400001 }, { github_pr: true, grant: "external_action" })
%w[list_fizzy_boards get_fizzy_board search_fizzy_cards get_fizzy_card create_fizzy_card comment_on_fizzy_card move_fizzy_card close_fizzy_card reopen_fizzy_card].each do |name|
  fields = { board_id: "abc", q: "hi", account_id: "acct", number: 1, title: "Title", body: "hi", column_id: "abc" }
  tool(name, fields, { grant: "read_messages" }, "fizzy_#{name}_grant")
  tool(name, fields, { grant: name.match?(/^(list|get|search)_/) ? "fizzy" : "external_action" }, "fizzy_#{name}_account")
end
tool("get_fizzy_card", { account_id: "acct", number: "abc" }, { grant: "fizzy", fizzy_account: true }, "fizzy_invalid_number")
tool("get_fizzy_board", { board_id: "bad/id" }, { grant: "fizzy", fizzy_account: true }, "fizzy_invalid_board")
tool("search_fizzy_cards", { q: " " }, { grant: "fizzy", fizzy_account: true }, "fizzy_empty_query")
contract("fizzy_action_invalid_kind", :post, "/agents/fizzy/card_actions", { kind: "bad", account_id: "bad/id", board_id: "bad/id" }, { grant: "external_action", fizzy_account: true })
contract("fizzy_action_create_fields", :post, "/agents/fizzy/card_actions", { kind: "create", title: " " }, { grant: "external_action", fizzy_account: true })
contract("fizzy_action_comment_fields", :post, "/agents/fizzy/card_actions", { kind: "comment", number: "abc", body: " " }, { grant: "external_action", fizzy_account: true })
contract("fizzy_action_move_fields", :post, "/agents/fizzy/card_actions", { kind: "move" }, { grant: "external_action", fizzy_account: true })
contract("fizzy_action_lengths", :post, "/agents/fizzy/card_actions", { kind: "create", board_id: "abc", title: "x" * 501, description: "x" * 3501, body: "x" * 3501 }, { grant: "external_action", fizzy_account: true })
contract("fizzy_action_budget", :post, "/agents/fizzy/card_actions", { kind: "close", number: 0 }, { grant: "external_action", fizzy_account: true, cap: 0 })
contract("fizzy_action_replay", :post, "/agents/fizzy/card_actions", { kind: "bad", external_id: "surface-replay" }, { grant: "external_action", fizzy_account: true, cap: 0, approval: "pending" })
contract("fizzy_unreadable_account", :get, "/agents/fizzy/boards", nil, { grant: "fizzy", fizzy_account: true, bad_fizzy_token: true })
contract("github_owner_pat_ignored", :post, "/rooms/486777696/agents/github/pull_request_actions", { pull_request_id: 900400001, kind: "bad" }, { grant: "external_action", github_pr: true, github_account: "owner_pat" })
contract("github_invalid_kind", :post, "/rooms/486777696/agents/github/pull_request_actions", { pull_request_id: 900400001, kind: "bad" }, { grant: "external_action", github_pr: true, github_account: "agent_pat" })
contract("github_comment_body", :post, "/rooms/486777696/agents/github/pull_request_actions", { pull_request_id: 900400001, kind: "comment", body: " " }, { grant: "external_action", github_pr: true, github_account: "owner_app" })
contract("github_reviewers", :post, "/rooms/486777696/agents/github/pull_request_actions", { pull_request_id: 900400001, kind: "request_review", reviewers: ["@bad--name"] }, { grant: "external_action", github_pr: true, github_account: "owner_app" })
contract("github_body_length", :post, "/rooms/486777696/agents/github/pull_request_actions", { pull_request_id: 900400001, kind: "comment", body: "x" * 3501 }, { grant: "external_action", github_pr: true, github_account: "agent_pat" })
contract("github_budget", :post, "/rooms/486777696/agents/github/pull_request_actions", { pull_request_id: 900400001, kind: "approve" }, { grant: "external_action", github_pr: true, github_account: "owner_app", cap: 0 })
contract("github_replay", :post, "/rooms/486777696/agents/github/pull_request_actions", { pull_request_id: 900400001, kind: "bad", external_id: "surface-replay" }, { grant: "external_action", github_pr: true, github_account: "owner_app", cap: 0, approval: "pending" })
contract("github_rate", :post, "/rooms/486777696/agents/github/pull_request_actions", { pull_request_id: 900400001, kind: "bad" }, { grant: "external_action", github_pr: true, github_account: "agent_pat", repeat: 60 })
tool("move_fizzy_card", { number: "abc", column_id: "bad/id" }, { grant: "external_action", fizzy_account: true }, "fizzy_move_invalid")
tool("close_fizzy_card", { number: 0 }, { grant: "external_action", fizzy_account: true, cap: 0 }, "fizzy_close_budget")
travel_to Time.utc(2026, 3, 2, 16) do
  agent = Agent.find(773018776)
  result = CASES.map do |item|
    agent.agent_approvals.delete_all
    AgentApproval.connection.execute("DELETE FROM sqlite_sequence WHERE name='agent_approvals'")
    agent.agent_grants.delete_all
    agent.agent_credentials.delete_all
    agent.update_columns(owner_id: item[:setup][:ownerless] ? nil : 127326141, daily_external_action_cap: item[:setup][:cap], daily_message_cap: item[:setup][:message_cap], daily_board_post_cap: nil)
    credential = agent.agent_credentials.create!(name: "Surface contract", created_by: User.find(127326141), token_digest: AgentCredential.digest(SECRET), token_last_four: AgentCredential.digest(SECRET).first(4))
    if item[:setup][:grant]
      Array(item[:setup][:grant]).each { |cap| AgentGrant.create!(agent: agent, capability: cap, granted_by: User.find(127326141)) }
    end
    if (status = item[:setup][:approval])
      approval = agent.agent_approvals.create!(id: 900100001, action: "deploy", summary: "Existing", room_id: 486777696, external_id: "surface-replay")
      approval.update_columns(status: status == "expired" ? "pending" : status, expires_at: status == "expired" ? 1.minute.ago : 24.hours.from_now)
    end
    Message.find(935961918).update_columns(creator_id: item[:setup][:own_message] ? 394959859 : 127326141, streaming: false)
    ChannelThread.where(id: 900200001).delete_all
    if item[:setup][:owned_work]
      temporary_grant = AgentGrant.create!(agent: agent, capability: "post_messages", granted_by: User.find(127326141))
      ChannelThread.create!(id: 900200001, name: "Owned", room_id: 486777696, creator_id: 394959859, work_owner_id: 394959859, work_status: "in_progress")
      temporary_grant.destroy!
    end
    FizzyConnectedAccount.delete_all
    GithubConnectedAccount.delete_all
    if item[:setup][:fizzy_account]
      FizzyConnectedAccount.create!(user_id: 127326141, fizzy_account_id: "acct", access_token: "ws11api-obviously-fake-fizzy")
    end
    if item[:setup][:bad_fizzy_token]
      FizzyConnectedAccount.connection.execute("UPDATE fizzy_connected_accounts SET access_token='unreadable-fixture'")
    end
    if (account = item[:setup][:github_account])
      GithubConnectedAccount.create!(user_id: account == "agent_pat" ? 394959859 : 127326141, github_login: "fixture", access_token: "ws11api-obviously-fake-github", token_source: account == "owner_app" ? "app" : "pat", token_expires_at: 1.hour.ago)
    end
    Github::PullRequestThread.where(github_pull_request_id: 900400001).delete_all
    Github::PullRequest.where(id: 900400001).delete_all
    ChannelThread.where(id: 900400002).delete_all
    if item[:setup][:github_pr]
      pr = Github::PullRequest.create!(id: 900400001, owner: "fixture", repo: "fixture", number: 1)
      temporary_grant = AgentGrant.create!(agent: agent, capability: "post_messages", granted_by: User.find(127326141))
      thread = ChannelThread.create!(id: 900400002, name: "PR", room_id: 486777696, creator_id: 394959859)
      temporary_grant.destroy!
      Github::PullRequestThread.create!(pull_request: pr, room_id: 486777696, channel_thread: thread)
    end
    Rails.cache = ActiveSupport::Cache::MemoryStore.new
    session = ActionDispatch::Integration::Session.new(Rails.application)
    session.host! "campfire.test"
    headers = { "Accept" => "application/json", "Content-Type" => "application/json", "Authorization" => ["Bearer", SECRET].join(" ") }
    raw = item[:body]&.to_json
    (item[:setup][:repeat] || 0).times { session.public_send(item[:method], item[:path], params: raw, headers: headers) }
    session.public_send(item[:method], item[:path], params: raw, headers: headers)
    response = session.response
    item.merge(body: raw, status: response.status, response: response.body.blank? ? nil : JSON.parse(response.body), response_headers: response.headers.slice("Cache-Control", "Pragma", "Retry-After"))
  end
  puts JSON.pretty_generate({ reference_pin: "d7c7de92", cases: result })
end
