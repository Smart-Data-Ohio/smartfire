class AddReaderVerifiedToGithubRepositorySubscriptions < ActiveRecord::Migration[8.2]
  def change
    # True when the subscriber's own linked GitHub account could read the
    # repository at subscription time. Unverified subscriptions (existing
    # rows and administrator overrides) never post private PR titles.
    add_column :github_repository_subscriptions, :reader_verified, :boolean, default: false, null: false
  end
end
