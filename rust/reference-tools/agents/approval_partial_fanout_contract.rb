ApplicationJob.queue_adapter = :test
agent=Agent.find(773018776);agent.update_columns(owner_id:nil)
User.active.without_bots.where(role: :administrator).each {|u| u.update_column(:inbox_preferences,{agent_approvals:true}.to_json)}
# Add two ordinary fixture administrators so the failure always has a prior recipient.
2.times {|n| User.create!(name:"WS11 fanout admin #{n}",email_address:"ws11-fanout-#{n}@example.test",role: :administrator,password:'ws11 fanout public fixture')}
conn=ActiveRecord::Base.connection
conn.execute("CREATE TEMP TRIGGER ws11_reject_later_recipient BEFORE INSERT ON activity_items WHEN NEW.source_type='AgentApproval' AND EXISTS(SELECT 1 FROM activity_items WHERE source_type='AgentApproval' AND source_id=NEW.source_id) BEGIN SELECT RAISE(ABORT,'later recipient failure'); END")
error=nil
begin
 AgentApproval.create!(agent:agent,action:'deploy',summary:'Partial fanout',external_id:'ws11-partial-fanout')
rescue => e
 error=e.class.name
end
approval=AgentApproval.find_by!(external_id:'ws11-partial-fanout')
result={error:error,approvals:AgentApproval.where(id:approval.id).count,inbox_items:ActivityItem.where(source:approval).count}
puts JSON.pretty_generate(result)
