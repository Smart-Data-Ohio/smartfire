# PR #176 callback registration and failure probe on Rails d7c7de92.
require 'active_support/testing/time_helpers'
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026,3,2,16)
agent=Agent.find(773018776)
room=Room.find(486777696)
human=User.find(127326141)
AgentGrant.delete_all
room.memberships.grant_to([agent.user])
make=->(name) { t=ChannelThread.create!(room:room,creator:human,name:name,work_status:'planned');t.update_columns(work_owner_id:agent.user_id);t }
observe=-> {
  cursor=0; titles=[]
  loop do
    page=Agents::EventPolling.poll(agent:agent,since:cursor,limit:1,presenter:->(m) {{id:m.id}})
    break if page[:events].empty?
    titles.concat(page[:events].map { |e| e[:work]['title'] || e[:work]['name'] })
    cursor=page[:next_since]
  end
  {ledger:agent.agent_events.order(:id).map { |e| e.metadata['title'] },polled:titles,threads_remaining:ChannelThread.where(name:['A','B','A edited']).count}
}
results={}
[false,true].each do |webhook|
  if webhook
    Webhook.find_or_create_by!(user:agent.user) { |w|w.url='https://bots.example.test/hook' }
  else
    Webhook.where(user:agent.user).delete_all
  end
  %w[plain updated_first created_same_transaction].each do |mode|
    agent.agent_events.delete_all
    if mode=='created_same_transaction'
      ActiveRecord::Base.transaction { a=make.call('A');b=make.call('B');b.destroy!;a.destroy! }
    else
      a=make.call('A');b=make.call('B')
      ActiveRecord::Base.transaction { a.update!(name:'A edited') if mode=='updated_first';b.destroy!;a.destroy! }
    end
    results["#{mode}_#{webhook}"]=observe.call
  end
end
[false,true].each do |webhook|
  if webhook
    Webhook.find_or_create_by!(user:agent.user) { |w|w.url='https://bots.example.test/hook' }
  else
    Webhook.where(user:agent.user).delete_all
  end
  agent.agent_events.delete_all
  a=make.call('A');b=make.call('B')
  ActiveRecord::Base.transaction do
    a.update!(name:'A edited')
    ActiveRecord::Base.transaction(requires_new:true) {c=make.call('Rolled back C');c.destroy!;raise ActiveRecord::Rollback}
    b.destroy!;a.destroy!
  end
  results["savepoint_#{webhook}"]=observe.call
end
Webhook.where(user:agent.user).delete_all
agent.agent_events.delete_all
a=make.call('A');b=make.call('B')
ActiveRecord::Base.connection.execute("CREATE TEMP TRIGGER ws11_reject_first BEFORE INSERT ON agent_events WHEN NEW.event_type='work_unassigned' AND json_extract(NEW.metadata,'$.thread_id')=#{b.id} BEGIN SELECT RAISE(ABORT,'WS11 rejected ledger'); END")
begin
  ActiveRecord::Base.transaction {b.destroy!;a.destroy!}
rescue ActiveRecord::StatementInvalid
  results[:first_ledger_failure_error]='statement_invalid'
ensure
  ActiveRecord::Base.connection.execute('DROP TRIGGER ws11_reject_first')
end
results[:first_ledger_failure]=observe.call
puts JSON.pretty_generate({reference:'d7c7de92',results:results}.as_json)
