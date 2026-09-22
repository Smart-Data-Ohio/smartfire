class Room::DestroyJob < ApplicationJob
  # Second half of asynchronous room deletion. Destroys a room marked by
  # Room#begin_destroy! in batches, with callbacks (dependent destroys,
  # reply tombstones, search index cleanup, calendar entry removal) instead
  # of delete_all. Idempotent: a retry resumes where the previous run
  # stopped, and a room that is gone — or was never marked — is a no-op.
  #
  # Messages go before threads on purpose: a thread's dependent destroy
  # loads its whole message set at once, so every message is destroyed in
  # batches first and each thread is already empty when its turn comes.
  BATCH_SIZE = 500

  # Rooms marked deleted longer ago than this that are still present get
  # their destroy re-enqueued: the original job never ran.
  STUCK_GRACE = 10.minutes

  # The destroy request marks the room inside a transaction; deferring the
  # enqueue to after commit keeps the job from running on pre-commit state.
  self.enqueue_after_transaction_commit = true

  # Re-enqueues destroys for rooms stuck as deleted. The destroy_enqueued_at
  # claim keeps this from re-enqueueing a destroy that is already queued or
  # running: only rooms never enqueued, or enqueued longer ago than the
  # grace, are claimed, and the claim update is conditional so two sweeps
  # racing each other enqueue only once.
  def self.reenqueue_stuck!(grace: STUCK_GRACE)
    cutoff = grace.ago
    Room.deleted.where(deleted_at: ...cutoff).destroy_unclaimed_before(cutoff).pluck(:id).each do |room_id|
      claimed = Room.where(id: room_id).destroy_unclaimed_before(cutoff).update_all(destroy_enqueued_at: Time.current)
      perform_later(room_id) if claimed == 1
    end
  end

  def perform(room_id)
    room = Room.find_by(id: room_id)
    return unless room&.deleted?

    # Hold the sweep claim while running, so a slow destroy is not
    # re-enqueued mid-flight. A duplicate would still be harmless: perform
    # is idempotent.
    room.update_columns(destroy_enqueued_at: Time.current)

    destroy_huddle_grants(room)
    room.messages.find_each(batch_size: BATCH_SIZE, &:destroy!)
    room.channel_threads.find_each(batch_size: BATCH_SIZE, &:destroy!)
    room.events.find_each(batch_size: BATCH_SIZE, &:destroy!)
    room.destroy!
  end

  private
    # Grants are revoked (not destroyed) by begin_destroy!; destroy them now
    # so no rows point at the deleted room. Cleanup rows keep their own
    # room_name/identity copies, so they survive the unlink and still run.
    def destroy_huddle_grants(room)
      grant_ids = HuddleGrant.where(room_id: room.id).pluck(:id)
      HuddleCleanup.where(huddle_grant_id: grant_ids).update_all(huddle_grant_id: nil)
      HuddleGrant.where(id: grant_ids).find_each(batch_size: BATCH_SIZE, &:destroy!)
    end
end
