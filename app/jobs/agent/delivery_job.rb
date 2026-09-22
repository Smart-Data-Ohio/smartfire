class Agent::DeliveryJob < ApplicationJob
  # Claim-guarded (the pending row is claimed atomically) and idempotent: a
  # retry would find no pending row and return. attempts: 1 opts out of the
  # inherited transient retries, so unexpected errors go straight to the
  # failure queue instead of retrying.
  retry_on(*ApplicationJob::TRANSIENT_ERRORS, attempts: 1)

  def perform(agent_event_id)
    event = AgentEvent.find_by(id: agent_event_id)
    Agent::Delivery.perform(event) if event
  end
end
