class CreateCalendarMeetingCaches < ActiveRecord::Migration[8.2]
  def change
    create_table :calendar_meeting_caches do |t|
      t.integer :user_id, null: false
      t.json :busy_intervals, null: false, default: []
      t.datetime :fetched_at
      t.string :fetch_error
      t.boolean :in_meeting_broadcast
      t.timestamps

      t.index :user_id, unique: true
    end

    add_foreign_key :calendar_meeting_caches, :users, column: :user_id, on_delete: :cascade
  end
end
