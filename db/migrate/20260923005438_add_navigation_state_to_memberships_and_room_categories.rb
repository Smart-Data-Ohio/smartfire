class AddNavigationStateToMembershipsAndRoomCategories < ActiveRecord::Migration[8.2]
  def change
    add_column :memberships, :last_read_message_id, :bigint
    add_column :memberships, :favorite_position, :integer
    add_column :memberships, :room_category_id, :bigint
    add_index :memberships, :last_read_message_id
    add_index :memberships, [ :user_id, :favorite_position ], name: "index_memberships_on_user_and_favorite"
    add_index :memberships, [ :user_id, :room_category_id ], name: "index_memberships_on_user_and_category"

    create_table :room_categories do |t|
      t.integer :user_id, null: false
      t.string :name, null: false
      t.integer :position, default: 0, null: false
      t.boolean :collapsed, default: false, null: false
      t.timestamps
      t.index [ :user_id, :position ], name: "index_room_categories_on_user_and_position"
      t.index :user_id, name: "index_room_categories_on_user_id"
    end
  end
end
