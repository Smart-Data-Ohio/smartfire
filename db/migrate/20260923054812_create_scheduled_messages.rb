class CreateScheduledMessages < ActiveRecord::Migration[8.2]
  def change
    create_table :scheduled_messages do |t|
      t.references :user, null: false, foreign_key: true
      t.references :room, null: false, foreign_key: true
      t.references :thread, foreign_key: { to_table: :channel_threads, on_delete: :nullify }
      t.references :reply_to_message, foreign_key: { to_table: :messages, on_delete: :nullify }
      t.references :sent_message, foreign_key: { to_table: :messages, on_delete: :nullify }
      t.text :markdown_source, null: false
      t.text :drop_reason
      t.datetime :send_at, null: false
      t.datetime :claimed_at
      t.datetime :sent_at
      t.datetime :dropped_at

      t.timestamps
    end
    add_index :scheduled_messages, %i[ user_id send_at ]
    add_index :scheduled_messages, %i[ send_at sent_at dropped_at ]
  end
end
