# Match the original system-test setup through the pinned Rails models, including
# parent timestamps, room bookkeeping, callback-created activity and rich text.
labels = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'parity/.seed/agents_ui/labels.json')))
Current.user = User.find(labels.fetch('users.david'))
agent = Agent.find(labels.fetch('agents.bender'))
room = Room.find(labels.fetch('rooms.watercooler'))
agent.update!(provider: 'OpenAI', runtime: 'Codex CLI 0.9', description: 'Does things', status: 'working', status_note: 'on it', last_seen_at: Time.current)
agent.set_working_presence!('Running tests…')
message = room.root_messages.create!(creator: agent.user, markdown_source: 'Working on it', client_message_id: 'sys-steps')
AgentStep.create!(agent:, message:, name: 'Run tests', status: 'done', output_summary: 'All green', duration_ms: 1500)
AgentStep.create!(agent:, message:, name: 'Deploy', status: 'running', input_summary: 'Ship it')
approval = AgentApproval.create!(agent:, room:, action: 'deploy', summary: 'Ship the release')
item = ActivityItem.find_by!(user: Current.user, source: approval)
%w[read_messages post_messages].each do |capability|
  unless agent.agent_grants.where(room: room, capability: capability, revoked_at: nil).exists?
    AgentGrant.create!(agent:, room:, capability:, granted_by: Current.user)
  end
end
_, secret = AgentCredential.create_with_secret!(agent:, name: 'Live UI replay', created_by: Current.user)
puts JSON.generate('system.streaming_secret' => secret, 'system.message' => message.id, 'system.message_dom_id' => ActionView::RecordIdentifier.dom_id(message), 'system.room' => room.id, 'system.approval' => approval.id, 'system.activity_item' => item.id)
