class AddGithubAppFieldsToGithubConnectedAccounts < ActiveRecord::Migration[8.2]
  def change
    add_column :github_connected_accounts, :token_source, :string, null: false, default: "pat"
    add_column :github_connected_accounts, :refresh_token, :string
    add_column :github_connected_accounts, :token_expires_at, :datetime
    add_column :github_connected_accounts, :last_error, :string
  end
end
