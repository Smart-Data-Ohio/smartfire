ApplicationJob.queue_adapter = :test
conn = ActiveRecord::Base.connection
conn.execute("CREATE TEMP TRIGGER ws11r_reject_inbox BEFORE INSERT ON activity_items WHEN NEW.source_type='AgentApproval' BEGIN SELECT RAISE(ABORT, 'ws11r inbox failure'); END")
error = nil
begin
  AgentApproval.create!(agent: Agent.find_by!(user_id: 394959859), action: 'deploy', summary: 'ws11r inbox failure', external_id: 'ws11r-inbox-failure')
rescue => e
  error = e.class.name
end
count = AgentApproval.where(external_id: 'ws11r-inbox-failure').count
puts "WS11R Rails approval inbox failure: error=#{error}; persisted approvals=#{count}"
raise 'Unexpected Rails behavior' unless error == 'ActiveRecord::StatementInvalid' && count == 1
