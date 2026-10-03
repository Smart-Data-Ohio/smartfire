# Original AgentWorkAssignmentTest setup; the browser supplies every work write.
labels = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'parity/.seed/agents_ui/labels.json')))
Current.user = User.find(labels.fetch('users.david'))
room = Room.find(labels.fetch('rooms.designers'))
bot = User.create_bot!(name: 'Work Agent')
agent = bot.create_agent!(kind: :workspace, owner: Current.user)
agent.update!(provider: 'TestLab', description: 'Does assigned work')
room.memberships.grant_to(bot)
%w[read_messages post_messages manage_threads].each do |capability|
 AgentGrant.create!(agent:, room:, granted_by: Current.user, capability:)
end
_, secret = AgentCredential.create_with_secret!(agent:, name: 'system', created_by: Current.user)
puts JSON.generate('system.work_bot' => bot.id, 'system.work_secret' => secret)
