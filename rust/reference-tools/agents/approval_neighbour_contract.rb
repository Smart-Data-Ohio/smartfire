require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter=:test
travel_to Time.utc(2026,3,2,16) do
  agent=Agent.find(773018776);owner=User.find(712064548)
  agent.update!(owner:owner);owner.update!(inbox_preferences:{"agent_approvals"=>false})
  approval=AgentApproval.create!(agent:agent,action:"deploy",summary:"ship it")
  html=ApplicationController.render partial:"users/mention",locals:{user:owner}
  body="Hey <action-text-attachment sgid=\"#{owner.attachable_sgid}\" content-type=\"application/vnd.campfire.mention\" content=\"#{html.gsub('"','&quot;')}\"></action-text-attachment>"
  mention=Room.find(699448326).messages.create!(creator_id:127326141,body:body,client_message_id:"approval-switch-neighbour")
  result={items:ActivityItem.where(source:approval).order(:user_id).pluck(:user_id),deciders:approval.deciders.map(&:id).sort,owner_can_decide:approval.decidable_by?(owner),mention:ActivityItem.find_by!(user:owner,source:mention).event_type}
  puts JSON.pretty_generate(reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],body:body,results:result)
end
