# Isolated setup for agent_streaming_test.rb's budget/kill-switch interaction.
# Subsequent posts and overflows go through each app's actual bot endpoint.
labels = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'parity/.seed/agents_ui/labels.json')))
agent = Agent.find(labels.fetch('agents.bender'))
Current.user = User.find(labels.fetch('users.kevin'))
agent.update!(owner: Current.user, daily_message_cap: 1, daily_board_post_cap: 10)
puts JSON.generate('system.room' => labels.fetch('rooms.watercooler'), 'system.bot_key' => agent.user.reset_bot_key)
