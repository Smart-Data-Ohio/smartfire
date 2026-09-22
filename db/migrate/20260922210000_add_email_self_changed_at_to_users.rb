class AddEmailSelfChangedAtToUsers < ActiveRecord::Migration[8.2]
  def change
    # Set when a member changes their own email address from the profile.
    # Google sign-in refuses to auto-link a first-time Google subject to an
    # account by email while this is set; an administrator clears it.
    add_column :users, :email_self_changed_at, :datetime
  end
end
