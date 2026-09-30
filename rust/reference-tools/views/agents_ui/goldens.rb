# Real Rails agent directory/status/thread-step rendering. No status or sort simulation.
require_relative "../bots_ui/setup"

david = User.find_by!(email_address: "david@37signals.com")
kevin = User.find_by!(email_address: "kevin@37signals.com")
bender = User.find_by!(name: "Bender Bot")
bender.agent.update!(status: "working", status_note: "Running <tests> & checks", last_seen_at: 5.minutes.ago, status_changed_at: 15.minutes.ago)
[["ΟΣ", :workspace, david, false], ["Ος", :workspace, david, false], ["Aaron Suspended", :workspace, david, true], ["Élodie Personal", :personal, kevin, false], ["Banned Bot", :workspace, david, false]].each do |name, kind, owner, suspended|
  bot = User.create_bot!(name:)
  agent = bot.create_agent!(kind:, owner:)
  agent.update!(suspended_at: Time.current) if suspended
  bot.update_columns(status: "banned") if name == "Banned Bot"
end
result = { pages: {}, agents: [] }
Agent.for_directory.each do |agent|
  result[:agents] << {
    id: agent.id, user_id: agent.user_id, name: agent.user.name,
    avatar_path: render_with(inline: "<%= fresh_user_avatar_path(bot) %>", locals: { bot: agent.user }),
    kind_description: agent.kind_description, status: agent.status, status_note: agent.status_note,
    suspended: agent.suspended?, created_at: agent.created_at.iso8601(6),
    status_changed_at: agent.status_changed_at&.iso8601(6), last_seen_at: agent.last_seen_at&.iso8601(6)
  }
end
[:admin, :member].each do |viewer|
  result[:pages][viewer] = render_with(user: viewer == :admin ? david : kevin, template: "agents/directory/index", layout: "application", assigns: { agents: Agent.for_directory })
end
result[:pages][:empty] = render_with(user: kevin, template: "agents/directory/index", layout: "application", assigns: { agents: [] })
result[:badges] = {}
bender.agent.tap do |agent|
  %w[idle working waiting failed suspended].each do |status|
    agent.status = status == "suspended" ? "failed" : status
    agent.suspended_at = status == "suspended" ? Time.current : nil
    agent.status_note = status == "idle" ? "  " : "Note <escaped>"
    agent.status_changed_at = nil if status == "waiting"
    result[:badges][status] = {
      facts: { id: agent.id, status: agent.status, status_note: agent.status_note, suspended: agent.suspended?,
               created_at: agent.created_at.iso8601(6), status_changed_at: agent.status_changed_at&.iso8601(6) },
      html: render_with(partial: "agents/status_badge", locals: { agent: })
    }
  end
end
steps = [nil, 0, 999, 1000, 1050, 1150, 9999].map.with_index do |duration, i|
  AgentStep.new(name: "Step <#{i}>", status: %w[pending running done failed][i % 4], duration_ms: duration, input_summary: "input\n<&>", output_summary: i.even? ? "  " : "output\n<&>")
end
thread = ChannelThread.new(id: 42)
thread.define_singleton_method(:agent_steps) { steps }
result[:steps] = steps.map { |step| step.attributes.slice("name", "status", "duration_ms", "input_summary", "output_summary") }
result[:thread_steps] = render_with(partial: "agent_steps/thread_steps", locals: { thread: })
thread.define_singleton_method(:agent_steps) { [] }
result[:thread_steps_empty] = render_with(partial: "agent_steps/thread_steps", locals: { thread: })
rng = Random.new(11)
durations = ((0..12000).to_a + (0..2000).flat_map { |n| [12000 + 50*n, 12049 + 50*n, 12051 + 50*n] } + Array.new(1000) { rng.rand(2**63) } + [2**53-1, 2**53, 2**53+50, 2**60-1, 2**63-1]).uniq
result[:durations] = durations.map { |duration| [duration, ApplicationController.helpers.agent_step_duration(duration)] }
File.write("/rails/storage/db/agents-ui.json", JSON.pretty_generate(result) + "\n")
puts "Rails agent UI goldens: #{result[:pages].size} pages, #{result[:badges].size} badges, 2 thread-step wrappers, #{result[:durations].size} durations"
