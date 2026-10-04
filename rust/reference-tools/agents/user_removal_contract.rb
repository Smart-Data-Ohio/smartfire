require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026, 3, 2, 16) do
  actor=User.find(127326141);room=Room.find(486777696)
  bot=User.create!(id:900150001,name:"Cleanup Bot",role: :bot,skip_open_room_grant:true)
  room.memberships.grant_to([bot])
  agent=Agent.create!(user:bot,owner:actor,kind: :workspace)
  credential=agent.agent_credentials.create!(name:"Keep",token_digest:"ws11-user-delete-digest",token_last_four:"disp",created_by:actor)
  grant=agent.agent_grants.create!(capability:"post_messages",granted_by:actor)
  bot.create_webhook!(url:"https://example.test/hook")
  own=room.messages.create!(id:900150010,creator:bot,markdown_source:"Remove me",client_message_id:"ws11-remove-own")
  other=room.messages.create!(id:900150011,creator:actor,markdown_source:"Survives",client_message_id:"ws11-remove-other")
  own_pin=MessagePin.create!(message:own,room:room,pinner:bot)
  other_pin=MessagePin.create!(message:other,room:room,pinner:bot)
  Boost.create!(message:other,booster:bot,content:"Yes")
  bot.saved_items.create!(message:other)
  thread=room.channel_threads.create!(id:900150020,creator:bot,name:"Remove thread")
  survivor=room.channel_threads.create!(id:900150021,creator:actor,name:"Keep owner work",work_status:"planned",work_owner:bot)
  bot.destroy!
  results={user:User.exists?(bot.id),agent:Agent.exists?(agent.id),webhook:Webhook.where(user_id:bot.id).count,
    credential:AgentCredential.exists?(credential.id),grant:AgentGrant.exists?(grant.id),grant_revoked:grant.reload.revoked?,
    own_message:Message.exists?(own.id),other_message:Message.exists?(other.id),pins:MessagePin.where(id:[own_pin.id,other_pin.id]).count,
    boosts:Boost.where(booster_id:bot.id).count,saves:SavedItem.where(user_id:bot.id).count,thread:ChannelThread.exists?(thread.id),
    survivor_owner:survivor.reload.work_owner_id,memberships:Membership.where(user_id:bot.id).count}
  puts JSON.pretty_generate({reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],results:results}.as_json)
end
