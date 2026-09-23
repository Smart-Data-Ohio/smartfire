require "digest"

# Indexed lookup for direct rooms by exact member set. Replaces the linear
# scan in Rooms::Direct.find_for: each direct room carries a hash of its
# sorted member ids, looked up through a partial unique index. Backfilled
# for existing direct rooms; only the new column is written.
#
# Two rooms can legitimately share a member set: adding or removing a
# member recomputes the key but never merges rooms, so a mutated group
# keeps its own history even when its set collides with another room's.
# The first room (alive rooms first, then lowest id) claims the plain key
# and every later claimant gets a room-suffixed key, which keeps the unique
# index satisfied while leaving exactly one room findable per set: the
# create/open-from-selection path reuses that room. Plain SQL plus Ruby's
# digest: no models, jobs, Redis, or network.
class AddDirectMemberKeyToRooms < ActiveRecord::Migration[8.2]
  def up
    add_column :rooms, :direct_member_key, :string

    member_ids_by_room = Hash.new { |hash, key| hash[key] = [] }
    # Alive rooms claim their plain key before deleted ones, so a deleted
    # room never shadows a live room with the same members.
    select_rows(<<~SQL.squish).each do |room_id, user_id|
      SELECT rooms.id, memberships.user_id FROM rooms
      INNER JOIN memberships ON memberships.room_id = rooms.id
      WHERE rooms.type = 'Rooms::Direct'
      ORDER BY rooms.deleted_at IS NOT NULL, rooms.id, memberships.user_id
    SQL
      member_ids_by_room[Integer(room_id)] << Integer(user_id)
    end

    claimed_keys = {}
    member_ids_by_room.each do |room_id, member_ids|
      key = "dm:#{Digest::SHA256.hexdigest(member_ids.join(","))}"
      key = "#{key}##{room_id}" if claimed_keys.key?(key)
      claimed_keys[key] = true

      execute "UPDATE rooms SET direct_member_key = #{quote(key)} WHERE id = #{room_id}"
    end

    # Deleted rooms hold stale keys outside the index, so deleting a group
    # never blocks recreating the same member set later.
    add_index :rooms, :direct_member_key, unique: true,
      where: "direct_member_key IS NOT NULL AND deleted_at IS NULL",
      name: "index_rooms_on_direct_member_key"
  end

  # Lossless: no pre-existing column was touched.
  # Rolls back with SQLite's native DROP COLUMN. remove_column would rebuild
  # the table inside the migration transaction, where foreign keys cannot be
  # switched off, so dropping the old table would fire ON DELETE actions on
  # the tables that reference it.
  def down
    remove_index :rooms, name: "index_rooms_on_direct_member_key"
    execute "ALTER TABLE rooms DROP COLUMN direct_member_key"
  end
end
