# The persisted false -> true transition fails before writing any content/stamp.
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026,3,2,16) do
  agent=Agent.find(773018776);room=Room.find(486777696)
  AgentGrant.delete_all;AgentEvent.delete_all
  message=room.messages.create!(creator:agent.user,streaming:true,markdown_source:"Draft",client_message_id:"ws11-resume-guard")
  message.finalize_stream!
  message.streaming=true
  message.markdown_source="Illicit resume"
  valid=message.valid?
  errors=message.errors[:streaming]
  saved=message.save
  persisted=Message.find(message.id)
  puts JSON.pretty_generate({reference_pin:"d7c7de92",results:{valid:valid,saved:saved,errors:errors,streaming:persisted.streaming?,source:persisted.markdown_source}})
end
