require 'active_support/testing/time_helpers'
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026,3,2,16) do
  agent=Agent.find(773018776);bot=agent.user;room=Room.find(486777696);actor=User.find(127326141)
  reset=-> { Message.where(creator:bot).update_all(created_at:Time.utc(2000,1,1));ChannelThread.where(creator:bot).update_all(created_at:Time.utc(2000,1,1));AgentApproval.where(agent:agent).delete_all }
  reset.call
  result={initial:Agents::Budgets.usage(agent)}
  room.messages.create!(creator:bot,markdown_source:'One',client_message_id:'ws11-budget-usage-one')
  board=Rooms::Board.create_for({name:'Budget Board',creator:actor},users:[actor,bot])
  ChannelThread.create_board_post!(room:board,creator:bot,name:'Post',work_status:'planned')
  AgentApproval.create!(agent:agent,action:'deploy',summary:'Ship it')
  result[:all]=Agents::Budgets.usage(agent)
  reset.call
  thread=ChannelThread.create_board_post!(room:board,creator:bot,name:'Opener',work_status:'planned',first_message:'Opening words')
  result[:opener]=Agents::Budgets.usage(agent)
  reply=thread.post_message!(creator:bot,attributes:{markdown_source:'A reply',client_message_id:'ws11-budget-usage-reply'})
  result[:reply]=Agents::Budgets.usage(agent)
  reply.update_columns(created_at:1.day.ago)
  result[:yesterday]=Agents::Budgets.usage(agent)
  puts JSON.pretty_generate(result)
end
