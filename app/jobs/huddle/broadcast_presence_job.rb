class Huddle::BroadcastPresenceJob < ApplicationJob
  # Renders and broadcasts the presence stacks for the grant's room. Split
  # out of the gateway's per-second liveness check so the check itself stays
  # a cheap read; participants are resolved when the job runs, so a grant
  # revoked in between still broadcasts the correct current stacks.
  def perform(grant_id)
    grant = HuddleGrant.find_by(id: grant_id)
    return unless grant

    grant.broadcast_voice_presence
  end
end
