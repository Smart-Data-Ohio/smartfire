class AddGithubIdentityToAgentApprovals < ActiveRecord::Migration[8.2]
  def change
    # The agent's linked GitHub account when a github.* action was
    # requested. Github::PerformAgentActionJob refuses to act through any
    # other account, so relinking the bot after approval cannot redirect
    # an approved action to a different GitHub identity.
    add_column :agent_approvals, :github_account_id, :integer
    add_column :agent_approvals, :github_login, :string
  end
end
