class AddPinsChangedAtToRooms < ActiveRecord::Migration[8.2]
  def change
    # Stamped on every pin and unpin (see MessagePin), so a reconnecting
    # client learns the pins count and panel went stale. NULL until the
    # room's first pin change; unpins destroy their row, so without this
    # stamp a refresh could never tell one happened.
    add_column :rooms, :pins_changed_at, :datetime
  end
end
