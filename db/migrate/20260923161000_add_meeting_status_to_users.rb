class AddMeetingStatusToUsers < ActiveRecord::Migration[8.2]
  def change
    add_column :users, :meeting_status_enabled, :boolean, default: false, null: false
    add_column :users, :meeting_dnd_enabled, :boolean, default: false, null: false
  end
end
