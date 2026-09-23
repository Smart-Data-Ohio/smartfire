class CreateFizzyConnectedAccounts < ActiveRecord::Migration[8.2]
  def change
    create_table :fizzy_connected_accounts do |t|
      t.references :user, null: false, index: { unique: true }, foreign_key: true
      t.string :access_token, null: false
      t.string :fizzy_account_id, null: false
      t.string :fizzy_account_name
      t.string :fizzy_user_id
      t.string :fizzy_user_name
      t.string :disconnected_reason

      t.timestamps
    end
  end
end
