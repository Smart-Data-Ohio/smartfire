ApplicationJob.queue_adapter = :test
agent=Agent.find(773018776);agent.agent_events.delete_all
base={agent:agent,event_type:'mention',outcome:'pending'}
invalid_type=AgentEvent.new(base.merge(event_type:'launch_missiles'));invalid_type.valid?
invalid_outcome=AgentEvent.new(base.merge(outcome:'bogus'));invalid_outcome.valid?
result={types:AgentEvent::EVENT_TYPES,invalid_type:invalid_type.errors.to_hash,invalid_outcome:invalid_outcome.errors.to_hash}
%w[mention direct_message reply approval_decided work_assigned work_unassigned github_action_completed].each {|type| AgentEvent.create!(base.merge(event_type:type))}
%w[delivery_suppressed_rate_limit delivery_suppressed_hop_limit delivery_suppressed_revoked posted].each {|type| AgentEvent.create!(base.merge(event_type:type,outcome:'suppressed'))}
result[:deliverable]=agent.agent_events.deliverable.pluck(:event_type).sort
result[:message_deliverable]=agent.agent_events.message_deliverable.pluck(:event_type).sort
result[:hop]=[AgentEvent.new.hop,AgentEvent.new(metadata:{'hop'=>2}).hop]
event=AgentEvent.create!(base.merge(outcome:'delivered'));event.acknowledged!;first=event.reload.outcome;event.acknowledged!
result[:acknowledgments]=[first,event.reload.outcome]
puts JSON.pretty_generate(result)
