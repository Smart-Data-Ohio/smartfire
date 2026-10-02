# Run through bin/rails runner against an isolated copy of the default seed.
# This replaces the complete repository-read seam and never contacts GitHub.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
Rails.logger = ActiveSupport::Logger.new(File::NULL)
PRIVATE_SECRET = "pr192-review-private-query-credential"
PRIVATE_AGENT = 773018776
PRIVATE_BOT = 394959859
PRIVATE_DAVID = 127326141
PRIVATE_ROOM = 486777696
PRIVATE_ACCOUNT = 1996100000
$private_repository_calls = 0
GithubConnectedAccount.prepend(Module.new do
  def can_read_repository?(owner, repo)
    raise "Unexpected repository identity" unless id == PRIVATE_ACCOUNT && user_id == PRIVATE_DAVID && owner == "review" && repo == "private"
    $private_repository_calls += 1
    true
  end
end)

def private_request(mcp)
  session = ActionDispatch::Integration::Session.new(Rails.application)
  session.host! "campfire.test"
  headers = {"Accept" => "application/json", "Content-Type" => "application/json", "Authorization" => "Bearer #{PRIVATE_SECRET}"}
  if mcp
    body = {jsonrpc: "2.0", id: 192, method: "tools/call", params: {name: "list_work", arguments: {}}}
    session.post("/agents/mcp", params: body.to_json, headers: headers)
  else
    session.get("/agents/work", headers: headers)
  end
  response = session.response
  raise "Status #{response.status}: #{response.body}" unless response.status == 200
  response
end

def private_measure(mcp, size)
  label = mcp ? "mcp_private_work" : "rest_private_work"
  warm = private_request(mcp)
  start_calls = $private_repository_calls
  statements = []
  subscriber = ->(*args) do
    event = args.last
    statements << event[:sql] if !event[:cached] && event[:sql].to_s.lstrip.match?(/\ASELECT\b/i)
  end
  response = nil
  ActiveSupport::Notifications.subscribed(subscriber, "sql.active_record") { response = private_request(mcp) }
  raise "Warm/current response changed" unless response.body == warm.body
  wire = JSON.parse(response.body)
  raise "Error: #{response.body}" if wire.is_a?(Hash) && (wire.key?("error") || wire.dig("result", "isError"))
  rows = mcp ? wire.dig("result", "structuredContent") : wire
  raise "Returned #{rows.size} expected #{size}" unless rows.size == size
  rows.each do |row|
    links = row.fetch("links")
    raise "Missing private payload: #{row}" unless links.size == 1 && links[0]["title"] == "Private title" && links[0].dig("pull_request", "head_branch") == "secret-branch"
  end
  puts "PR192_PRIVATE_RAILS label=#{label} size=#{size} returned=#{rows.size} select_count=#{statements.size} reader_calls=#{$private_repository_calls - start_calls}"
  statements.map { |sql| sql.gsub(/'[^']*'|\b\d+\b/, "?") }.tally.sort_by { |sql, count| [-count, sql] }.each do |sql, count|
    puts "PR192_PRIVATE_SQL label=#{label} size=#{size} count=#{count} sql=#{sql}"
  end
end

travel_to Time.utc(2026, 3, 2, 16) do
  executor = Rails.application.executor.run!(reset: true)
  begin
    ActiveRecord::Base.transaction do
      agent = Agent.find(PRIVATE_AGENT)
      agent.agent_events.delete_all
      agent.agent_grants.delete_all
      agent.agent_credentials.delete_all
      agent.update_columns(owner_id: PRIVATE_DAVID, status: "idle", status_note: nil, status_changed_at: nil, working_presence: nil, working_presence_expires_at: nil, last_seen_at: nil)
      digest = AgentCredential.digest(PRIVATE_SECRET)
      agent.agent_credentials.create!(name: "PR192 private query probe", created_by_id: PRIVATE_DAVID, token_digest: digest, token_last_four: digest.first(4))
      GithubConnectedAccount.where(user_id: PRIVATE_DAVID).delete_all
      GithubConnectedAccount.create!(id: PRIVATE_ACCOUNT, user_id: PRIVATE_DAVID, github_login: "review-owner", access_token: "obviously-fake-review-token", token_source: "pat")
      connection = ActiveRecord::Base.connection
      connection.execute("UPDATE channel_threads SET work_owner_id=NULL WHERE work_owner_id=#{PRIVATE_BOT}")
      Rails.cache = ActiveSupport::Cache::MemoryStore.new
      [[0, 5], [5, 50]].each do |start, finish|
        (start...finish).each do |n|
          thread, pr, link = 1996101000 + n, 1996102000 + n, 1996103000 + n
          connection.execute("INSERT INTO channel_threads(id,name,room_id,creator_id,work_owner_id,work_status,last_activity_at,created_at,updated_at) VALUES(#{thread},'Private work query probe',#{PRIVATE_ROOM},#{PRIVATE_DAVID},#{PRIVATE_BOT},'in_progress','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')")
          connection.execute("INSERT INTO github_pull_requests(id,owner,repo,number,title,state,head_branch,base_branch,private,created_at,updated_at) VALUES(#{pr},'review','private',#{n+1},'Private title','open','secret-branch','main',1,'2026-03-02 16:00:00','2026-03-02 16:00:00')")
          connection.execute("INSERT INTO work_thread_links(id,channel_thread_id,kind,github_pull_request_id,created_by_id,created_at,updated_at) VALUES(#{link},#{thread},'pull_request',#{pr},#{PRIVATE_DAVID},'2026-03-02 16:00:00','2026-03-02 16:00:00')")
        end
        private_measure(false, finish)
        private_measure(true, finish)
      end
      raise ActiveRecord::Rollback
    end
  ensure
    executor.complete!
  end
end
