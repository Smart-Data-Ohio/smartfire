class CreateUserStars < ActiveRecord::Migration[8.2]
  def change
    create_table :user_stars do |t|
      t.references :user, null: false, foreign_key: { on_delete: :cascade }
      t.references :starred_user, null: false, foreign_key: { to_table: :users, on_delete: :cascade }

      t.timestamps
    end

    add_index :user_stars, %i[ user_id starred_user_id ], unique: true
  end
end
