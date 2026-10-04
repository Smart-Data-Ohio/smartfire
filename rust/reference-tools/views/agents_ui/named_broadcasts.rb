# The three UI-owned names, through the real models/dispatcher and test pubsub used by Rails tests.
require 'action_cable/subscription_adapter/test'
ActionCable.server.config.cable={'adapter'=>'test'}
ActionCable.server.restart
ActiveRecord::Base.logger=nil
Rails.logger=ActiveSupport::Logger.new($stderr)
bot=User.find(394959859);agent=bot.agent;human=User.find(127326141);room=Room.find(486777696)
frames=->(streams){streams.flat_map { |stream| ActionCable.server.pubsub.broadcasts(stream).map { |raw| {stream:,html:JSON.parse(raw)} } }}
clear=-> { ActionCable.server.pubsub.clear }
clear.call
agent.update!(status:'working',status_note:'on it')
status=frames.call(['agents:all'])
raise 'status callback must emit two frames' unless status.size==2
clear.call
AgentGrant.create!(agent:,granted_by:human,capability:'post_messages')
clear.call
agent.update!(status:'failed')
secrets=frames.call(['agents:all'])
private_values=[AgentCredential.find(800727228).token_digest,'post_messages']
raise 'secret fixture leaks' if secrets.any? { |f| private_values.any? { |s| f[:html].include?(s) } }
clear.call
human.update_columns(time_zone:'America/New_York',ooo_until:nil,ooo_note:nil)
Current.user=human.reload
Time.use_zone('America/New_York') do
 result=SlashCommands::Dispatcher.dispatch(user:human,room:,text:'/ooo tomorrow Back soon')
 raise "OOO did not dispatch: #{result.inspect}" unless result.kind==:ephemeral
end
streams=[[human,:status],[human,:ooo_notice]].map { |s| Turbo::StreamsChannel.send(:stream_name_from,s) }
ooo=frames.call(streams)
raise "OOO must emit a badge and a notice: #{ooo.inspect}" unless ooo.size==2
puts JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA"),at:Time.current.iso8601,status:,secrets:,private_values:,ooo:)
warn 'Rails named UI broadcasts: 3 comparisons, 6 complete stream frames'
