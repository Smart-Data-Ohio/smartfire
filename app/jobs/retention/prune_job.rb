class Retention::PruneJob < ApplicationJob
  # How long each row kind is kept. All deletes run in batches; every branch
  # is idempotent, so a retry resumes instead of redoing work.
  AGENT_EVENTS_RETENTION = 90.days
  ACTIVITY_ITEMS_RETENTION = 180.days
  WEBHOOK_DELIVERIES_RETENTION = 14.days
  CLEANUPS_RETENTION = 7.days
  GRANTS_RETENTION = 30.days
  AUDIT_LOGS_RETENTION = 1.year
  FIZZY_CARD_CACHES_RETENTION = 1.day
  # Rooms marked deleted longer ago than this that are still present get
  # their destroy re-enqueued: the original job never ran.
  STUCK_ROOM_GRACE = 1.hour

  def perform
    AgentEvent.where(created_at: ...AGENT_EVENTS_RETENTION.ago).in_batches.delete_all
    prune_activity_items
    Github::WebhookDelivery.where(created_at: ...WEBHOOK_DELIVERIES_RETENTION.ago).in_batches.delete_all
    HuddleCleanup.where.not(completed_at: nil).where(completed_at: ...CLEANUPS_RETENTION.ago).in_batches.delete_all
    # The audit log's only delete path: rows are otherwise append-only.
    AuditLog.where(created_at: ...AUDIT_LOGS_RETENTION.ago).in_batches.delete_all
    Fizzy::CardCache.where(updated_at: ...FIZZY_CARD_CACHES_RETENTION.ago).in_batches.delete_all
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
    # The unlink runs once per batch of grants, not once per grant.
    def prune_huddle_grants
      HuddleGrant.where.not(revoked_at: nil).where(revoked_at: ...GRANTS_RETENTION.ago).in_batches do |batch|
        grant_ids = batch.pluck(:id)
        HuddleCleanup.where(huddle_grant_id: grant_ids).update_all(huddle_grant_id: nil)
        HuddleGrant.where(id: grant_ids).find_each(&:destroy!)
      end
    end

    # The periodic runner sweeps stuck destroys every few minutes; this
    # daily pass is the backstop with a longer grace.
    def reenqueue_stuck_room_destroys
      Room::DestroyJob.reenqueue_stuck!(grace: STUCK_ROOM_GRACE)
    end
end
