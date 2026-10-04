# Original named API declarations. Production requests and domain producers commit;
# fixture changes between requests model human assignment and permission revocation.
require "json"
require "nokogiri"
source = File.read(File.join(ENV.fetch("PARITY_WORK"), "reference-tools/agents/next5_work_named.rb"))
source = source.sub('next5-work-named-inputs.json', ARGV.fetch(0, 'next6-named-inputs.json'))
source = source.sub('last = row.fetch(:steps).last', 'last = row.fetch(:steps).reverse.find { |s| s[:method] }')
source = source.sub('observations = []', <<'SETUP'.chomp)
observations = []
requests = []
ChannelThread.where(id: thread).update_all(work_status:"planned") if setup[:initial_planned]
ChannelThread.where("id < 1900700020",work_owner_id:394959859).update_all(work_owner_id:nil)
AgentCredential.create!(agent: receiver, name: "Receiver contract", created_by_id: 127326141, token_digest: AgentCredential.digest("ws11api-receiver-credential"), token_last_four: "test")
if setup[:hide_base]
  ChannelThread.where(id: thread).update_all(work_owner_id: 127326141, work_status: "done", room_id: 201306877)
  Message.where(thread_id: thread).update_all(room_id: 201306877)
end
# No network is contacted during assignment. Only the external dial is substituted
# when the committed Agent::EventWebhookJob is explicitly executed below.
transport = Object.new
transport.define_singleton_method(:request) do |request|
  requests << {body: request.body, timestamp: request["X-Smartfire-Timestamp"], signature: request["X-Smartfire-Signature"], content_type: request["Content-Type"]}
  response = Net::HTTPOK.new("1.1", "200", "OK"); response.instance_variable_set(:@read, true); response.body = ""; response
end
Net::HTTP.define_singleton_method(:start) do |host, port, **options, &block|
  raise "unapproved network" unless host == "93.184.216.34" && port == 8080 && options[:ipaddr] == "93.184.216.34"
  block.call(transport)
end
SETUP
source = source.sub('  session = ActionDispatch::Integration::Session.new(Rails.application)', <<'ACTIONS'.chomp)
  if step[:action]
    action_execution = Rails.application.executor.run!(reset: true)
    begin
    output = nil
    case step.fetch(:action)
    when "post"
      conn.execute("UPDATE sqlite_sequence SET seq=#{step.fetch(:id)-1} WHERE name='channel_threads'")
      ChannelThread.create_board_post!(room: Room.find(step.fetch(:room, room)), creator: User.find(127326141), name: step.fetch(:title), work_status: step.fetch(:status), owner_id: step.fetch(:owner), first_message: step[:body])
    when "age"
      ChannelThread.find(step.fetch(:id)).update_columns(last_activity_at: Time.current - step.fetch(:hours).hours, updated_at: Time.current - step.fetch(:hours).hours)
    when "reply"
      ChannelThread.find(step.fetch(:id)).post_message!(creator: User.find(127326141), attributes: {markdown_source: step.fetch(:text)})
    when "assign"
      target = ChannelThread.find(step.fetch(:id))
      target.update_columns(work_status: nil, work_owner_id: nil, name: step.fetch(:title), creator_id: 127326141)
      ThreadMembership.join!(target, User.find(127326141))
      target.update_work!(actor: User.find(127326141), work_status: "planned", work_owner_id: 394959859)
    when "unassign", "reassign"
      ChannelThread.find(step.fetch(:id)).update_work!(actor: User.find(127326141), work_owner_id: step[:action] == "unassign" ? nil : 394959859)
    when "links"
      target = ChannelThread.find(step.fetch(:id))
      if step[:pr]
        pr = Github::PullRequest.for_reference(owner: "rails", repo: "rails", number: 7)
        pr.update!(private: false, title: "Fix login", state: "open", html_url: "https://github.com/rails/rails/pull/7")
        target.work_thread_links.create!(kind: :pull_request, github_pull_request: pr, created_by_id: 127326141)
      end
      target.work_thread_links.create!(kind: :event, event: Event.find_by!(title: "Watercooler sync"), created_by_id: 127326141)
      target.work_thread_links.create!(kind: :drive_file, url: "https://drive.google.com/file/d/1AbcDefGhIjKlMnOpQrSt/view", title: step[:drive_title], created_by_id: 127326141) unless step[:event_only]
    when "join"
      Room.find(step.fetch(:room)).memberships.grant_to(agent.user)
    when "leave"
      Membership.find_by!(room_id: step.fetch(:room), user_id: agent.user_id).destroy!
    when "revoke"
      scope = agent.agent_grants.where(capability: step.fetch(:cap))
      scope = scope.where(room_id: step[:room]) if step[:room]
      scope.each(&:revoke!)
    when "read_elsewhere"
      AgentGrant.create!(agent: agent, room_id: 201306877, capability: "read_messages", granted_by_id: 127326141)
    when "hook"
      target = Agent.find(step.fetch(:agent, agent.id))
      target.user.webhook.update!(url: "http://93.184.216.34:8080/hook", signing_secret: "ws11-next6-public-signing-material")
      target.update!(webhook_signing_secret: "ws11-next6-public-signing-material")
    when "deliver"
      raise "assignment blocked on external HTTP" unless requests.empty?
      queued = ApplicationJob.queue_adapter.enqueued_jobs.select { |j| j[:job] == Agent::EventWebhookJob && AgentEvent.find(j[:args][0]).agent_id == step.fetch(:agent) }
      queued.each do |j|
        j[:job].perform_now(*ActiveJob::Arguments.deserialize(j[:args]))
        ApplicationJob.queue_adapter.enqueued_jobs.delete(j)
      end
      output = {queued: queued.size, requests: requests.dup}
    when "mention"
      mention_agent = Agent.find(step.fetch(:agent,agent.id))
      step[:bodies] = (step.fetch(:offset, 0)...step.fetch(:offset, 0)+step.fetch(:count)).map { |n| "Ping #{n} <action-text-attachment sgid=\"#{mention_agent.user.attachable_sgid}\" content-type=\"application/vnd.campfire.mention\"></action-text-attachment>" }
      step[:bodies].each_with_index { |body, n| Room.find(room).messages.create!(creator_id: 127326141, body: body, client_message_id: "work-rate-#{step.fetch(:offset, 0)+n}") }
    when "inbox"
      items = ActivityItem.where(user_id: 127326141, source_type: "WorkThreadEvent", source_id: WorkThreadEvent.where("id > ?", before[:history]).select(:id)).order(:id)
      html_session = ActionDispatch::Integration::Session.new(Rails.application); html_session.host! "campfire.test"
      cookie = JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "vectors/campfire_sessions.json"))).fetch("sessions").find { |s| s.fetch("user_name") == "David" }.fetch("cookie_header")
      cookie.split("; ").each { |part| key, value = part.split("=", 2); html_session.cookies[key] = CGI.unescape(value) }
      html_session.get("/activity", headers: {"Accept" => "text/html"})
      doc = Nokogiri::HTML(html_session.response.body)
      output = {status: html_session.response.status, items: items.map { |i| {id: i.id, source_id: i.source_id, event_type: i.event_type, present: !!doc.at_css("#activity_item_#{i.id}")} }}
    else raise "unknown action #{step[:action]}"
    end
    observations << step.merge(output: output) if output
    ensure
      action_execution.complete!
    end
    next
  end
  session = ActionDispatch::Integration::Session.new(Rails.application)
ACTIONS
source = source.sub('  request_body = step[:body]&.to_json', '  select_start = queries.size' + "\n  request_body = step[:body]&.to_json")
source = source.sub('request_body: request_body, status:', 'request_body: request_body, selects: queries.size-select_start, cache_hits: 0, status:')
source = source.sub('  case step[:auth]' , '  request_headers["Accept"] = "text/html" if step[:projection]' + "\n  case step[:auth]")
source = source.sub('  when "session"', <<'AUTH'.chomp)
  when "receiver"
    request_headers["Authorization"] = ["Bearer", "ws11api-receiver-credential"].join(" ")
  when "session"
AUTH
source = source.sub('  observations << step.merge(request_body:', <<'PROJECTION'.chomp)
  response_body = response.body
  if step[:projection]
    doc = Nokogiri::HTML(response_body)
    text = ->(selector) { doc.css(selector).map { |node| node.text.split.join(" ") }.join(" ") }
    response_body = case step[:projection]
    when "human_created" then {text:doc.text.split.join(" ")}.to_json
    when "history" then JSON.parse(response_body).fetch("thread").fetch("work_history").to_json
    when "note" then {note: doc.text.include?("Digging into the bug")}.to_json
    when "board_row" then {title: text.call("#board_row_channel_thread_1901300001").include?("Ship the launch"), owner: text.call("#board_row_channel_thread_1901300001 .board-row__owner").include?("Bender Bot"), badge: text.call("#board_row_channel_thread_1901300001 .agent-badge")}.to_json
    when "post" then {brief: text.call(".board-post__messages").include?("Everything goes out Friday."), run: doc.css(".board-post__run a").map { |n| [n["href"], n.text.split.join(" ")] }}.to_json
    when "result" then {reply: text.call(".board-post__messages").include?("Halfway there."), result: text.call(".board-post__result-body"), history: text.call(".board-post__history").include?("Bender Bot updated the result")}.to_json
    else raise "unknown projection" end
  end
  observations << step.merge(request_body:
PROJECTION
source = source.sub('response_body: response.body,', 'response_body: response_body,')
source = source.sub('item[:observations] = observations', <<'EXTRA'.chomp)
item[:observations] = observations
item[:extra_state] = {
  threads: ChannelThread.where("id >= 1900700020").order(:id).map { |t| {id:t.id, title:t.name, room_id:t.room_id, owner:t.work_owner_id, status:t.work_status, result:t.result_markdown, count:t.messages_count, updated_at:stamp.call(t.updated_at), activity_at:stamp.call(t.last_activity_at)} },
  messages: Message.where("id > 1900700003").order(:id).map { |m| {id:m.id, thread_id:m.thread_id, creator_id:m.creator_id, markdown:m.markdown_source, opener:m.board_post_opener} },
  history: WorkThreadEvent.where("id > ?",before[:history]).order(:id).map { |e| {id:e.id, thread_id:e.channel_thread_id, note:e.note} },
  handoffs: WorkHandoff.where("id > 1901302000").order(:id).map { |h| {id:h.id, thread_id:h.channel_thread_id, summary:h.summary} },
  ledger: AgentEvent.where("id > ?",before[:ledger]).order(:id).map { |e| {id:e.id, status:e.webhook_status, attempts:e.webhook_attempts, outcome:e.outcome} },
  inbox: ActivityItem.where(source_type:"WorkThreadEvent",source_id:WorkThreadEvent.where("id > ?",before[:history]).select(:id)).order(:id).map { |i| {id:i.id,user_id:i.user_id,source_id:i.source_id,event_type:i.event_type} }
}
EXTRA
source = source.sub('%w[Content-Type Cache-Control Pragma Retry-After Location].to_h', '(step[:projection] ? (step[:projection]=="human_created" ? %w[Content-Type Location] : %w[Content-Type]) : %w[Content-Type Cache-Control Pragma Retry-After Location X-Smartfire-Next-Since]).to_h')
# Disable caching at the real request executor boundary, rather than wrapping a
# request in uncached and then letting its executor re-enable the query cache.
Rails.application.executor.to_run { ActiveRecord::Base.connection_handler.each_connection_pool.each(&:disable_query_cache!) }
source = source.sub('eval(source, TOPLEVEL_BINDING, "next5 named committed oracle")', <<'JOBS'.chomp)
source = replace_once(source, 'callback=->(*args){payload=args.last;queries<<payload[:sql] if !payload[:cached] && payload[:sql].match?(/\ASELECT\b/i)}', 'callback=->(*args){payload=args.last;if payload[:sql].match?(/\ASELECT\b/i) && payload[:name] != "SCHEMA";raise "query cache enabled at request boundary" if payload[:cached] || ActiveRecord::Base.connection.query_cache_enabled;queries<<payload[:sql];end}')
source = replace_once(source, 'WorkHandoff.where(channel_thread_id:written.id)', 'WorkHandoff.where("id > 1901302000")')
source = replace_once(source, 'when "ChannelThread::PushMessageJob" then', 'when "Room::PushMessageJob" then {room_id:j[:args][0]["_aj_globalid"].split("/").last.to_i,message_id:j[:args][1]["_aj_globalid"].split("/").last.to_i}' + "\n          when \"ChannelThread::PushMessageJob\" then")
source = replace_once(source, 'when "Agent::EventWebhookJob" then', 'when "Agent::DeliveryJob" then {event_id:j[:args][0]}' + "\n          when \"Agent::EventWebhookJob\" then")
source = replace_once(source, 'Ledger chains are checked for a shared UUID independently because production UUIDs are random; no response fields are masked.', 'Every production chain is a UUID; one handoff pair shares a chain. Raw REST/MCP bodies are unmasked. Human HTML/history projections reproduce the original declarations. Rate-case logical jobs are compared as a complete multiset, retaining duplicate class/argument facts; the original case does not specify queue execution order.')
# The parent harness's last-response fields duplicate observations, and for HTML
# include unrelated random CSRF/client IDs. Store only the responses we assert.
source = replace_once(source, 'results<<item.merge(body:body,status:response.status,response_body:response.body,response_headers:%w[Content-Type Cache-Control Pragma Retry-After Location].to_h{|key|[key,response.headers[key]]},state:state,selects:queries.size)', 'results << item.except(:name,:surface,:method,:path,:body).merge(state:state)')
eval(source, TOPLEVEL_BINDING, "next5 named committed oracle")
JOBS
eval(source, TOPLEVEL_BINDING, "next6 named committed oracle")
rows = TOPLEVEL_BINDING.eval("results")
observations = rows.flat_map { |row| row.fetch(:observations) }
requests = observations.count { |row| row[:method] }
raise "query cache hit" unless observations.all? { |row| row.fetch(:cache_hits, 0).zero? }
warn "WS11 next6 Rails oracle: #{rows.size} cases; #{requests} HTTP responses; #{observations.size-requests} producer outputs; 0 query cache hits"
