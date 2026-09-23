class AddDeletedAtToRooms < ActiveRecord::Migration[8.2]
  def change
    # Soft-delete marker for asynchronous room destruction: the destroy
    # request stamps this and removes memberships, then Room::DestroyJob
    # removes the room and its content in batches.
    add_column :rooms, :deleted_at, :datetime
  end
end
