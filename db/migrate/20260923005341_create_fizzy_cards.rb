class CreateFizzyCards < ActiveRecord::Migration[8.2]
  def change
    create_table :fizzy_cards do |t|
      t.string :account_id, null: false
      t.integer :number, null: false

      t.timestamps

      t.index %i[ account_id number ], unique: true
    end

    create_table :fizzy_card_references do |t|
      t.references :message, null: false, foreign_key: true
      t.references :fizzy_card, null: false, foreign_key: true

      t.timestamps

      t.index %i[ message_id fizzy_card_id ], unique: true, name: "index_fizzy_card_refs_on_message_and_card"
    end

    create_table :fizzy_card_caches do |t|
      t.references :fizzy_card, null: false, foreign_key: true
      t.references :user, null: false, foreign_key: true
      t.json :payload
      t.datetime :fetched_at
      t.string :fetch_error
      t.datetime :fetch_requested_at

      t.timestamps

      t.index %i[ fizzy_card_id user_id ], unique: true, name: "index_fizzy_card_caches_on_card_and_user"
    end
  end
end
