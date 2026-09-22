class AddEmailSelfChangedAtToUsers < ActiveRecord::Migration[8.2]
  def up
    # Set when a member changes their own email address from the profile.
    # Google sign-in refuses to auto-link a first-time Google subject to an
    # account by email while this is set; an administrator clears it.
    add_column :users, :email_self_changed_at, :datetime
  end

  # Rolls back with SQLite's native DROP COLUMN. remove_column would rebuild
  # the table inside the migration transaction, where foreign keys cannot be
  # switched off, so dropping the old table would fire ON DELETE actions on
  # the tables that reference it.
  def down
    execute "ALTER TABLE users DROP COLUMN email_self_changed_at"
  end
end
