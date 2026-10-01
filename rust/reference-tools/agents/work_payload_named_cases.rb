require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter=:test
travel_to Time.utc(2026,3,2,16) do
  room=Room.find(486777696);bot=User.find(394959859);human=User.find(127326141);agent=Agent.find(773018776)
  AgentGrant.create!(agent:agent,room:room,granted_by:human,capability:"post_messages")
  thread=ChannelThread.create!(id:900150001,room:room,creator:human,name:"Payload work")
  ThreadMembership.join!(thread,human)
  thread.update_work!(actor:human,work_status:"in_progress",work_owner_id:bot.id)
  thread.update_result!(actor:human,markdown:"## Outcome")
  results={thread:Agents::WorkPayload.for(thread.reload)}
  human_thread=ChannelThread.create!(id:900150002,room:room,creator:human,name:"Human work")
  ThreadMembership.join!(human_thread,human)
  human_thread.update_work!(actor:human,work_status:"planned",work_owner_id:149087659)
  unowned=ChannelThread.create!(id:900150003,room:room,creator:human,name:"Unowned work")
  ThreadMembership.join!(unowned,human)
  unowned.update_work!(actor:human,work_status:"planned")
  results[:human]=Agents::WorkPayload.for(human_thread.reload)
  results[:unowned]=Agents::WorkPayload.for(unowned.reload)
  board=Rooms::Board.create_for({id:900150010,name:"Launch",creator:human},users:[human])
  board.memberships.grant_to(bot)
  AgentGrant.create!(agent:agent,room:board,granted_by:human,capability:"post_messages")
  post=ChannelThread.create_board_post!(room:board,creator:bot,name:"Board payload",work_status:"in_progress",owner_id:bot.id,tags:"api, launch",run_url:"https://example.com/runs/1",first_message:"Brief.")
  results[:board]=Agents::WorkPayload.for(post.reload)
  puts JSON.pretty_generate({reference_pin:"d7c7de92",results:results}.as_json)
end
