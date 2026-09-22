class Room::DestroyJob < ApplicationJob
  # Second half of asynchronous room deletion. Destroys a room marked by
  # Room#begin_destroy! in batches, with callbacks (dependent destroys,
  # reply tombstones, search index cleanup, calendar entry removal) instead
  # of delete_all. Idempotent: a retry resumes where the previous run
  # stopped, and a room that is gone — or was never marked — is a no-op.
  # (A run that lands before its marking transaction commits either blocks
  # on the write lock and retries, or no-ops and is picked up again by the
  # retention sweep for rooms that stay marked.)
  #
  # Messages go before threads on purpose: a thread's dependent destroy
  # loads its whole message set at once, so every message is destroyed in
  # batches first and each thread is already empty when its turn comes.
  BATCH_SIZE = 500

  def perform(room_id)
    room = Room.find_by(id: room_id)
    return unless room&.deleted?

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
