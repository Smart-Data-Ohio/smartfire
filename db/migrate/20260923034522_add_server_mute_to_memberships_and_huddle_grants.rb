class AddServerMuteToMembershipsAndHuddleGrants < ActiveRecord::Migration[8.2]
  def change
    add_column :memberships, :server_muted_at, :datetime
    add_column :huddle_grants, :server_muted, :boolean, default: false, null: false
  end
end
