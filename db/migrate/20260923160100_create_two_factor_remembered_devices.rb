class CreateTwoFactorRememberedDevices < ActiveRecord::Migration[8.2]
  # Revocable "remember this device" records backing the signed remember
  # cookie, plus the per-session flag recording that a session completed
  # the second factor (or was issued by the test-only sign-in route).
  def change
    create_table :two_factor_remembered_devices do |t|
      t.references :user, null: false, foreign_key: true
      t.string :token_digest, null: false
      t.string :user_agent, limit: 512
      t.string :ip_address
      t.datetime :expires_at, null: false
      t.datetime :last_used_at
      t.timestamps
    end
    add_index :two_factor_remembered_devices, :token_digest, unique: true

    add_column :sessions, :two_factor_verified_at, :datetime
  end
end
