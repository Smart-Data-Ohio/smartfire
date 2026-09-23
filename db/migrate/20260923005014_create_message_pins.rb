class CreateMessagePins < ActiveRecord::Migration[8.2]
  def change
    create_table :message_pins do |t|
      t.references :message, null: false, foreign_key: true, index: { unique: true }
      t.references :room, null: false, foreign_key: true
      t.references :pinner, null: false, foreign_key: { to_table: :users }

      t.timestamps
    end

    add_index :message_pins, %i[ room_id created_at ]
  end
end
