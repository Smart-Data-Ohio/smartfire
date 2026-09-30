require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026, 3, 2, 16) do
  agent = Agent.find(773018776)
  owner = User.find(127326141)
  human = User.find_by!(name: "Jason")
  base = { user: human, owner: owner, kind: "workspace" }
  validation = {}
  {
    missing_user: { user: nil }, personal_owner: { owner: nil, kind: "personal" },
    workspace_owner: { owner: nil }, duplicate_user: { user: agent.user },
    bad_status: { status: "napping" }, long_note: { status_note: "é" * 201 },
    long_description: { description: "é" * 501 }, long_presence: { working_presence: "é" * 141 }, presence_boundary: { working_presence: "é" * 140 },
    zero_messages: { daily_message_cap: 0 }, negative_board: { daily_board_post_cap: -1 },
    zero_actions: { daily_external_action_cap: 0 }, valid: { daily_message_cap: 1 }
  }.each do |name, fields|
    row = Agent.new(base.merge(fields)); row.valid?; validation[name] = row.errors.to_hash
  end
  agent.agent_grants.delete_all
  reads = { defaults: { kind: Agent.new.kind, status: Agent.new.status }, legacy: agent.grants_summary }
  %w[watercooler designers hq].each do |label|
    room = Room.find_by!(name: { "watercooler" => "All Talk", "designers" => "Designers", "hq" => "HQ" }.fetch(label))
    agent.agent_grants.create!(capability: "post_messages", room: room, granted_by: owner)
  end
  agent.agent_grants.create!(capability: "read_messages", granted_by: owner)
  reads[:grants] = agent.grants_summary
  agent.agent_grants.find_each(&:revoke!)
  reads[:revoked] = agent.grants_summary
  agent.update!(kind: "personal", owner: User.find_by!(name: "Kevin"))
  reads[:personal] = agent.kind_description
  agent.update!(kind: "workspace")
  reads[:workspace] = agent.kind_description
  agent.update_columns(owner_id: nil)
  reads[:ownerless] = agent.reload.kind_description
  reads[:ownerless_valid] = agent.valid?
  agent.update!(description: "Backfill")
  frames = []
  ActionCable.server.define_singleton_method(:broadcast) { |stream, data, **_| frames << { stream: stream, target: data.to_s[/target="([^"]+)"/, 1] } }
  status = {}
  capture = lambda do |name, operation|
    frames.clear; operation.call
    status[name] = { changed_at: agent.reload.status_changed_at, broadcasts: frames.dup }
  end
  capture.call(:note, -> { agent.update!(status_note: "Ready") })
  capture.call(:provider, -> { agent.update!(provider: "Acme") })
  capture.call(:working, -> { agent.update!(status: "working", status_note: "Now") })
  capture.call(:no_change, -> { agent.update!(status: "working") })
  capture.call(:seen, -> { agent.touch_last_seen! })
  presence = {}
  [ [ :set, " Thinking… " ], [ :clear, " \t\0" ], [ :unicode, "\u00a0Thinking\u00a0" ] ].each do |name, text|
    agent.set_working_presence!(text)
    presence[name] = { stored: agent.working_presence, expires_at: agent.working_presence_expires_at, text: agent.working_presence_text }
  end
  presence[:boundary] = agent.working_presence_text(now: Time.current + 5.minutes)
  presence[:before] = agent.working_presence_text(now: Time.current + 5.minutes - Rational(1, 1_000_000))
  agent.agent_events.delete_all
  [ ["mention", "delivered", Time.current], ["reply", "acknowledged", Time.current], ["posted", "delivered", Time.current], ["delivery_suppressed_rate_limit", "suppressed", Time.current], ["mention", "delivered", Time.current - 25.hours] ].each do |type, outcome, created|
    agent.agent_events.create!(event_type: type, outcome: outcome, created_at: created)
  end
  reads[:activity] = agent.activity_summary
  agent.update!(suspended_at: Time.current)
  reads[:suspended_grants] = agent.agent_grants.where(revoked_at: nil).count
  reads[:active] = agent.active?
  reads[:suspended] = agent.suspended?
  agent.update_columns(suspended_at: nil)
  [["Ágent", :active, false], ["Zed Suspended", :active, true], ["Aaron Gone", :deactivated, false], ["Banned", :banned, false]].each do |name, state, suspended|
    bot = User.create_bot!(name: name)
    row = bot.create_agent!(kind: :workspace, owner: owner)
    row.update!(suspended_at: Time.current) if suspended
    bot.update_column(:status, state)
  end
  reads[:directory] = Agent.for_directory.map { |row| row.user.name }
  puts JSON.pretty_generate({ reference_pin: "d7c7de92", validation: validation, reads: reads, status: status, presence: presence }.as_json)
end
