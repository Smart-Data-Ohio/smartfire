class CreateMessageReferences < ActiveRecord::Migration[8.2]
  def change
    create_table :message_references do |t|
      t.references :message, null: false, foreign_key: true
      t.references :referenced_message, null: false, foreign_key: { to_table: :messages }

      t.timestamps
    end

    add_index :message_references, %i[message_id referenced_message_id],
      unique: true, name: "index_message_references_on_message_and_referenced"
  end
end
