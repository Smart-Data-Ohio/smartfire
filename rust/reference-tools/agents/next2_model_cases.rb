# Remaining WS11 model comparisons, using the installed WS12 producers/viewer.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter=:test
Rails.logger=ActiveSupport::Logger.new($stderr)
kind=ARGV.fetch(0)
travel_to Time.utc(2026,3,2,16) do
  room=Room.find(486777696);human=User.find(127326141);bot=User.find(394959859);agent=Agent.find(773018776)
  AgentGrant.delete_all;AgentEvent.delete_all
  room.memberships.grant_to(bot)
  ActiveRecord::Base.connection.execute("UPDATE sqlite_sequence SET seq=1901820000 WHERE name='channel_threads'")
  case kind
  when "create_bot","reset_key"
    entropy="5M0aLYwQyBXOXa5Wsz6NZb11EE4tW2"
    original=SecureRandom.method(:alphanumeric)
    calls=[]
    SecureRandom.define_singleton_method(:alphanumeric) { |length| calls << length;entropy }
    created=User.create_bot!(id:1901800001,name:"Bender")
    first=created.bot_key
    entropy="R4kme9anwWRuz3sSoBXiB8Li8ioZPP"
    second=created.reset_bot_key if kind=="reset_key"
    stored=User.find(created.id)
    result={id:created.id,name:stored.name,role:stored.role,status:stored.status,first:first,second:second,
      plaintext:stored.read_attribute(:bot_token),digest:stored.bot_token_digest,stored_key:stored.bot_key,
      first_auth:User.authenticate_bot(first)&.id,second_auth:second ? User.authenticate_bot(second)&.id : nil,entropy_calls:calls}
    SecureRandom.define_singleton_method(:alphanumeric,original)
  when "hop_limit","human_root"
    board=Rooms::Board.create_for({id:1901810001,name:"Loop Board",creator:human},users:[human])
    bots=[1901800011,1901800021].each_with_index.map do |id,i|
      user=User.create_bot!(id:id,name:"Loop Agent #{i==0 ? 'A' : 'B'}")
      entry=user.create_agent!(id:id+1,kind: :workspace,owner:human)
      board.memberships.grant_to(user);entry
    end
    actors=kind=="human_root" ? [human] : [bots[0].user,bots[1].user,bots[0].user,bots[1].user]
    actors.each_with_index do |actor,i|
      owner=bots[1-i%2].user_id
      owner=bots[0].user_id if kind=="human_root"
      ChannelThread.create_board_post!(room:board,creator:actor,name:"Post #{i+1}",work_status:"in_progress",owner_id:owner)
    end
    events=AgentEvent.order(:id).map { |e| e.attributes.slice("agent_id","event_type","outcome","actor_id","hop","detail","webhook_status").merge("metadata"=>e.metadata) }
    result={events:events,threads:ChannelThread.where(id:1901820001..1901820004).order(:id).pluck(:id,:creator_id,:work_owner_id),same_chain:AgentEvent.pluck(:chain_id).uniq.length==1,
      history:WorkThreadEvent.where(channel_thread_id:1901820001..1901820004).order(:id).pluck(:channel_thread_id,:event_type,:actor_id,:from_owner_id,:to_owner_id)}
  when "delete_no_owner"
    thread=ChannelThread.create!(id:1901820001,room:room,creator:human,name:"Agent work")
    thread.destroy!
    result={exists:ChannelThread.exists?(thread.id),events:AgentEvent.count,jobs:ApplicationJob.queue_adapter.enqueued_jobs.select{|j|j[:job]==Agent::EventWebhookJob}.length}
  when "budget_viewer"
    Message.where(creator:bot).update_all(created_at:20.years.ago)
    agent.update!(daily_message_cap:1)
    room.root_messages.create!(creator:bot,markdown_source:"One",client_message_id:"budget-stranger")
    denial=Agents::Budgets.check(agent,:messages)
    result={status:denial.status.to_s,error:denial.error,
      stranger:ActivityItem.accessible_to(User.find(149087659)).where(event_type:"agent_budget_exceeded").count,
      owner:ActivityItem.accessible_to(human).where(event_type:"agent_budget_exceeded").count}
  when "budget_handoff"
    Message.where(creator:bot).update_all(created_at:20.years.ago)
    ChannelThread.where(creator:bot).update_all(created_at:20.years.ago)
    AgentApproval.where(agent:agent).delete_all
    board=Rooms::Board.create_for({id:1901810001,name:"Handoff Board",creator:human},users:[human,bot])
    receiver_bot=User.create_bot!(id:1901800011,name:"Handoff Receiver")
    receiver=receiver_bot.create_agent!(id:1901800012,kind: :workspace,owner:human)
    board.memberships.grant_to(receiver_bot)
    [agent,receiver].each { |a| %w[read_messages post_messages manage_threads].each{|cap|AgentGrant.create!(agent:a,room:board,granted_by:human,capability:cap)} }
    first=ChannelThread.create_board_post!(room:board,creator:human,name:"First",work_status:"in_progress",owner_id:bot.id)
    second=ChannelThread.create_board_post!(room:board,creator:human,name:"Second",work_status:"in_progress",owner_id:bot.id)
    one=Agents::WorkHandoffs.create(agent:agent,id:first.id,receiver_agent_id:receiver.id,summary:"Halfway there")
    initial=Agents::Budgets.usage(agent)
    agent.update!(daily_message_cap:1,daily_board_post_cap:1,daily_external_action_cap:1)
    room.root_messages.create!(creator:bot,markdown_source:"One",client_message_id:"budget-handoff-exhaust")
    Agents::Budgets.check(agent,:messages)
    two=Agents::WorkHandoffs.create(agent:agent,id:second.id,receiver_agent_id:receiver.id,summary:"Still yours")
    result={first_ok:one.ok?,second_ok:two.ok?,initial_usage:initial,final_usage:Agents::Budgets.usage(agent),owner:second.reload.work_owner_id,
      handoffs:WorkHandoff.where(channel_thread_id:[first.id,second.id]).order(:id).pluck(:channel_thread_id,:sender_id,:receiver_agent_id,:summary)}
  else
    raise "unknown case #{kind}"
  end
  puts JSON.pretty_generate(name:kind,result:result)
end
