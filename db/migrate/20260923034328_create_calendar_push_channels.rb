class CreateCalendarPushChannels < ActiveRecord::Migration[8.2]
  def change
    create_table :calendar_push_channels do |t|
      t.references :user, null: false, foreign_key: true, index: false
      t.string :channel_id, null: false
      t.string :resource_id
      t.string :token_digest, null: false
      t.datetime :expires_at
      t.bigint :last_message_number, null: false, default: 0
      t.datetime :last_notification_at
      t.string :last_error

      t.timestamps

      t.index :channel_id, unique: true
    end
    add_index :calendar_push_channels, :user_id, unique: true
  end
end
