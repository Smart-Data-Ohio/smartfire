# A real Rails finalize with one failing peer callback. The CAS and earlier
# effects survive the exception; repeating finalize cannot fan out again.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026,3,2,16) do
  agent=Agent.find(773018776);room=Room.find(486777696)
  AgentGrant.delete_all;AgentEvent.delete_all
  agent.update_columns(suspended_at:nil)
  message=room.messages.create!(creator:agent.user,streaming:true,markdown_source:"Before activity failure",client_message_id:"ws11-finalization-failure")
  agent.set_working_presence!("Finishing")
  def message.record_activity_items
    raise "WS12 adapter rejected activity"
  end
  error=begin;message.finalize_stream!;nil;rescue=>e;e.message;end
  results={error:error,streaming:message.reload.streaming?,indexed:Message.connection.select_value("SELECT COUNT(*) FROM message_search_index WHERE rowid=#{message.id}").to_i,
    posted:agent.agent_events.where(event_type:"posted",message_id:message.id).count,presence:agent.reload.working_presence,repeated:message.finalize_stream!}
  puts JSON.pretty_generate({reference_pin:"d7c7de92",results:results}.as_json)
end
