class AddLastActiveAtToWorkspacePresenceLeases < ActiveRecord::Migration[8.1]
  def change
    add_column :workspace_presence_leases, :last_active_at, :datetime
  end
end
