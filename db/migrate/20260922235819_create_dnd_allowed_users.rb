class CreateDndAllowedUsers < ActiveRecord::Migration[8.1]
  def change
    create_table :dnd_allowed_users do |t|
      t.references :user, null: false, foreign_key: true
      t.references :allowed_user, null: false, foreign_key: { to_table: :users }

      t.timestamps
    end

    add_index :dnd_allowed_users, %i[ user_id allowed_user_id ], unique: true
  end
end
