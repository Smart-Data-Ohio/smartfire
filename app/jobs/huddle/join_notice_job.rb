class Huddle::JoinNoticeJob < ApplicationJob
  # Fans a first-sighting join out to the room's members: in-call toasts
  # plus, in DMs, outsider banners with a throttled push. Split out of
  # the gateway's per-second liveness check like the presence broadcast,
  # so the check itself stays a cheap read.
  def perform(grant_id)
    grant = HuddleGrant.find_by(id: grant_id)
    return unless grant

    Huddle::JoinNotifier.notify_join(grant)
  end
end
