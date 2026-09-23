class CreateSavedItems < ActiveRecord::Migration[8.2]
  def change
    create_table :saved_items do |t|
      t.references :user, null: false, foreign_key: true
      t.references :message, null: false, foreign_key: true
      t.string :status, null: false, default: "in_progress"
      t.datetime :remind_at
      t.datetime :reminded_at

      t.timestamps
    end

    add_index :saved_items, %i[ user_id message_id ], unique: true
    add_index :saved_items, %i[ user_id status ]
    add_index :saved_items, :remind_at, where: "remind_at IS NOT NULL AND reminded_at IS NULL"
  end
end
