class Retention::PruneJob < ApplicationJob
  # How long each row kind is kept. All deletes run in batches; every branch
  # is idempotent, so a retry resumes instead of redoing work.
  AGENT_EVENTS_RETENTION = 90.days
  ACTIVITY_ITEMS_RETENTION = 180.days
  WEBHOOK_DELIVERIES_RETENTION = 14.days
  CLEANUPS_RETENTION = 7.days
  GRANTS_RETENTION = 30.days
  # Rooms marked deleted longer ago than this that are still present get
  # their destroy re-enqueued: the original job never ran.
  STUCK_ROOM_GRACE = 1.hour

  def perform
    AgentEvent.where(created_at: ...AGENT_EVENTS_RETENTION.ago).in_batches.delete_all
    prune_activity_items
    Github::WebhookDelivery.where(created_at: ...WEBHOOK_DELIVERIES_RETENTION.ago).in_batches.delete_all
    HuddleCleanup.where.not(completed_at: nil).where(completed_at: ...CLEANUPS_RETENTION.ago).in_batches.delete_all
    prune_huddle_grants
    reenqueue_stuck_room_destroys
  end

  private
    # Only items the user has seen (read or handled — handling always marks
    # read too). Unread items are never pruned, however old: they are
    # notifications the user has not seen yet. updated_at — not created_at —
    # is the age: grouped updates refresh an item in place, and an item the
    # user just marked read is still live context.
    def prune_activity_items
      ActivityItem.where.not(read_at: nil)
        .where(updated_at: ...ACTIVITY_ITEMS_RETENTION.ago)
        .in_batches.delete_all
    end

    # Revoked grants go through destroy (not delete_all) so their inbox
    # items go with them. Cleanup rows keep their own room_name/identity
    # copies, so unlinking a grant never strands pending reconcile work.
    def prune_huddle_grants
      HuddleGrant.where.not(revoked_at: nil).where(revoked_at: ...GRANTS_RETENTION.ago).find_each do |grant|
        HuddleCleanup.where(huddle_grant_id: grant.id).update_all(huddle_grant_id: nil)
        grant.destroy!
      end
    end

    # A destroy already in flight makes this a harmless duplicate:
    # Room::DestroyJob is idempotent.
    def reenqueue_stuck_room_destroys
      Room.where.not(deleted_at: nil).where(deleted_at: ...STUCK_ROOM_GRACE.ago).pluck(:id).each do |room_id|
        Room::DestroyJob.perform_later(room_id)
      end
    end
end
