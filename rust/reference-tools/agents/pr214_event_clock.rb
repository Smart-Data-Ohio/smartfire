# Returned/persisted webhook schedule equality under a real advancing clock.
require 'json'
Rails.logger=ActiveSupport::Logger.new($stderr)
ActiveJob::Base.queue_adapter=:test
base=Time.utc(2027,1,15,8); calls=0
original=Time.method(:current)
Time.define_singleton_method(:current){calls+=1; base+Rational(calls,1_000_000)}
begin
 rows=[]
 ActiveRecord::Base.transaction do
  agent=Agent.find(773018776)
  AgentGrant.where(agent_id:agent.id).delete_all
  conn=ActiveRecord::Base.connection
  conn.execute("INSERT OR IGNORE INTO memberships(room_id,user_id,created_at,updated_at) VALUES(486777696,394959859,'2026-03-02 16:00:00','2026-03-02 16:00:00')")
  direct=agent.agent_events.create!(event_type:'approval_decided',outcome:'delivered',webhook_status:'pending',webhook_next_attempt_at:Time.current)
  conn.execute("INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_owner_id,last_activity_at,created_at,updated_at) VALUES(2106930000,486777696,127326141,'Dynamic clock','planned',394959859,'2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')")
  thread=ChannelThread.find(2106930000)
  work=thread.send(:record_work_assignment_events!,from_owner:nil,to_owner:User.find(394959859),actor:User.find(127326141)).first
  thread.send(:deliver_work_assignment_webhooks,[work])
  [direct,work].each do |event|
   stored=AgentEvent.find(event.id)
   rows << {kind:event.event_type,returned:event.webhook_next_attempt_at&.iso8601(6),stored:stored.webhook_next_attempt_at&.iso8601(6),webhook_status:event.webhook_status,equal:event.webhook_next_attempt_at==stored.webhook_next_attempt_at,delta_microseconds:((event.webhook_next_attempt_at-stored.webhook_next_attempt_at)*1_000_000).round}
  end
  raise ActiveRecord::Rollback
 end
 puts JSON.pretty_generate(reference_pin:'d7c7de92',rows:rows,calls:calls)
ensure
 Time.define_singleton_method(:current,original)
end
