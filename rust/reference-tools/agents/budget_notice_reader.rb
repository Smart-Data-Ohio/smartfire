# Read-only exported AgentBudgetNotice facts and current recipient policy.
require 'active_support/testing/time_helpers'
extend ActiveSupport::Testing::TimeHelpers
Rails.logger=ActiveSupport::Logger.new($stderr)
travel_to Time.utc(2026,3,2,16) do
 results={}
 %w[owner inactive_owner bot_owner ownerless].each do |kind|
  ActiveRecord::Base.transaction do
   agent=Agent.find(773018776)
   agent.update_columns(owner_id: kind=='ownerless' ? nil : kind=='bot_owner' ? 394959859 : 127326141,daily_message_cap:2,daily_board_post_cap:nil,daily_external_action_cap:7)
   User.find(127326141).update_columns(status:1) if kind=='inactive_owner'
   rows=AgentBudgetNotice::CAPS.each_with_index.map do |cap,i|
    notice=AgentBudgetNotice.create!(id:2106900000+i,agent:agent,cap:cap,day:Date.new(2026,3,2))
    {id:notice.id,agent_id:notice.agent_id,cap:notice.cap,day:notice.day.iso8601,cap_label:notice.cap_label,budget_limit:notice.budget_limit,recipients:notice.activity_recipient_ids.sort,created_at:notice.created_at.iso8601(6),updated_at:notice.updated_at.iso8601(6)}
   end
   results[kind]=rows
   raise ActiveRecord::Rollback
  end
 end
 puts JSON.pretty_generate(reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],results:results,missing_id:nil)
end
