class AddDestroyEnqueuedAtToRooms < ActiveRecord::Migration[8.2]
  def change
    # Sweep claim for asynchronous room destruction: every Room::DestroyJob
    # enqueue stamps this, and a running job refreshes it, so the
    # stuck-room sweep never re-enqueues a destroy that is already queued
    # or running. NULL for rooms marked before this column existed; the
    # sweep treats those as unclaimed.
    add_column :rooms, :destroy_enqueued_at, :datetime
  end
end
