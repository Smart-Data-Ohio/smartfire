# Valid Rails model setup for the system cases that create fixtures directly.
# Run against each isolated app storage copy before its browser sequence.
require "json"
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
travel_to(Time.iso8601(ARGV.fetch(1, "2026-03-02T16:00:00Z"))) do
labels = JSON.parse(File.read(ARGV.fetch(0)))
room = Room.find(labels.fetch("rooms.designers"))
Current.user = User.find(labels.fetch("users.david"))
room.message_pins.destroy_all
Current.user.saved_items.destroy_all
agent = Agent.find(labels.fetch("agents.bender"))
room.memberships.grant_to(agent.user)
AgentSlashCommand.where(agent:, room:, name: %w[deploy ship later]).destroy_all
AgentSlashCommand.create!(agent:, room:, name: "deploy", description: "Ship it")
AgentSlashCommand.create!(agent:, room:, name: "ship", takes_arguments: false)
# polls_test.rb bypasses validation to model a once-future close time becoming due.
poll = Poll.find(labels.fetch("polls.closed"))
poll.update_columns(closes_at: Time.current - 1.minute, closed_at: nil)
puts "WS8bm2 browser fixtures: 2 validated agent commands; 1 expired poll; agent membership granted"
end
