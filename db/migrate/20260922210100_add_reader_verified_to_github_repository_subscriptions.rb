class AddReaderVerifiedToGithubRepositorySubscriptions < ActiveRecord::Migration[8.2]
  def up
    # True when the subscriber's own linked GitHub account could read the
    # repository at subscription time. Unverified subscriptions (existing
    # rows and administrator overrides) never post private PR titles.
    add_column :github_repository_subscriptions, :reader_verified, :boolean, default: false, null: false
  end

  # Rolls back with SQLite's native DROP COLUMN. remove_column would rebuild
  # the table inside the migration transaction, where foreign keys cannot be
  # switched off, so dropping the old table would fire ON DELETE actions on
  # the tables that reference it.
  def down
    execute "ALTER TABLE github_repository_subscriptions DROP COLUMN reader_verified"
  end
end
