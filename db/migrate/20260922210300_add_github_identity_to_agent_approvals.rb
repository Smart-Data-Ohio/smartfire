class AddGithubIdentityToAgentApprovals < ActiveRecord::Migration[8.2]
  def up
    # The agent's linked GitHub account when a github.* action was
    # requested. Github::PerformAgentActionJob refuses to act through any
    # other account, so relinking the bot after approval cannot redirect
    # an approved action to a different GitHub identity.
    add_column :agent_approvals, :github_account_id, :integer
    add_column :agent_approvals, :github_login, :string
  end

  # Rolls back with SQLite's native DROP COLUMN. remove_column would rebuild
  # the table inside the migration transaction, where foreign keys cannot be
  # switched off, so dropping the old table would fire ON DELETE actions on
  # the tables that reference it.
  def down
    execute "ALTER TABLE agent_approvals DROP COLUMN github_login"
    execute "ALTER TABLE agent_approvals DROP COLUMN github_account_id"
  end
end
