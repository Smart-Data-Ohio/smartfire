# Timeline entries the workspace posts itself: group-DM renames, joins,
# and leaves. Rendered centered without an author; never mention, reply,
# or notify anyone. Plain column only; existing rows read false.
class AddSystemToMessages < ActiveRecord::Migration[8.2]
  def up
    add_column :messages, :system, :boolean, default: false, null: false
  end

  # Lossless: no pre-existing column was touched.
  # Rolls back with SQLite's native DROP COLUMN. remove_column would rebuild
  # the table inside the migration transaction, where foreign keys cannot be
  # switched off, so dropping the old table would fire ON DELETE actions on
  # the tables that reference it.
  def down
    execute "ALTER TABLE messages DROP COLUMN system"
  end
end
