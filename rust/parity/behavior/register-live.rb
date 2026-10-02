# Mirrors slash_commands_test.rb's model fixture created after the page loaded.
require "json"
labels = JSON.parse(File.read(ARGV.fetch(0)))
room = Room.find(labels.fetch("rooms.designers"))
agent = Agent.find(labels.fetch("agents.bender"))
Current.user = User.find(labels.fetch("users.jz"))
room.memberships.grant_to(agent.user)
AgentSlashCommand.create!(agent:, room:, name: "later")
puts "WS8bm2 live browser fixture: 1 validated agent command registered after page load"
