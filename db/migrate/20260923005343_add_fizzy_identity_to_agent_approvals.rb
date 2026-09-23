class AddFizzyIdentityToAgentApprovals < ActiveRecord::Migration[8.2]
  def change
    change_table :agent_approvals do |t|
      t.integer :fizzy_connected_account_id
      t.string :fizzy_user_id
      t.string :fizzy_user_name
    end

    add_index :agent_approvals, :fizzy_connected_account_id
  end
end
