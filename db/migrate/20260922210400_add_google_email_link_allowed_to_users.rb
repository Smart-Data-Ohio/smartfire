class AddGoogleEmailLinkAllowedToUsers < ActiveRecord::Migration[8.2]
  # Google sign-in may link a first-time Google subject to an account by
  # email only when that email is known to be the person's own. That holds
  # for accounts that existed before this release and still carry their
  # original email; any account created later (join-code signups included)
  # chose its email itself, so it never auto-links and links from its own
  # profile instead (or an administrator allows it). Plain SQL only.
  def up
    add_column :users, :google_email_link_allowed, :boolean, default: false, null: false

    execute <<~SQL
      UPDATE users SET google_email_link_allowed = 1
      WHERE role != 2 AND email_self_changed_at IS NULL
        AND email_address IS NOT NULL AND email_address != ''
    SQL
  end

  def down
    remove_column :users, :google_email_link_allowed
  end
end
