class AddStatusAndNotificationSettingsToUsers < ActiveRecord::Migration[8.1]
  def change
    change_table :users, bulk: true do |t|
      t.string :presence_setting, null: false, default: "auto"
      t.string :custom_status_emoji
      t.string :custom_status_text
      t.datetime :custom_status_expires_at
      t.boolean :dnd_enabled, null: false, default: false
      t.boolean :quiet_hours_enabled, null: false, default: false
      t.integer :quiet_hours_start_minute
      t.integer :quiet_hours_end_minute
      t.string :time_zone
      t.string :theme, null: false, default: "system"
    end
  end
end
