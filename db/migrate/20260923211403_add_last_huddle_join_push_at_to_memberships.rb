class AddLastHuddleJoinPushAtToMemberships < ActiveRecord::Migration[8.2]
  def change
    add_column :memberships, :last_huddle_join_push_at, :datetime
  end
end
