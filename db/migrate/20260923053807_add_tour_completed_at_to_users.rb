# First-run tour bookkeeping: existing members have already seen the app,
# so they start stamped and the tour never auto-starts for them; members
# created after this column appears start NULL and see it once. Plain SQL
# on the new column only: no models, jobs, Redis, or network.
class AddTourCompletedAtToUsers < ActiveRecord::Migration[8.2]
  def up
    add_column :users, :tour_completed_at, :datetime
    execute("UPDATE users SET tour_completed_at = CURRENT_TIMESTAMP")
  end

  # Lossless: the column never existed before this release. Rolls back
  # with SQLite's native DROP COLUMN (see DigestBotTokens).
  def down
    execute "ALTER TABLE users DROP COLUMN tour_completed_at"
  end
end
