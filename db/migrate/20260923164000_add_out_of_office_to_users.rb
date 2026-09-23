class AddOutOfOfficeToUsers < ActiveRecord::Migration[8.2]
  def change
    add_column :users, :ooo_until, :datetime
    add_column :users, :ooo_note, :string, limit: 140
    add_column :users, :ooo_broadcast, :boolean
    add_column :users, :ooo_calendar_enabled, :boolean, default: false, null: false
    add_column :users, :ooo_notify_enabled, :boolean, default: false, null: false
  end
end
