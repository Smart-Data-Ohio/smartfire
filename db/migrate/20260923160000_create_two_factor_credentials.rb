class CreateTwoFactorCredentials < ActiveRecord::Migration[8.2]
  # TOTP credentials plus single-use backup codes. The secret is encrypted
  # at rest by Active Record encryption (see TwoFactorCredential); backup
  # codes are stored as SHA-256 digests only. last_totp_at holds the unix
  # time of the last accepted TOTP step for replay protection.
  def change
    create_table :two_factor_credentials do |t|
      t.references :user, null: false, index: { unique: true }, foreign_key: true
      t.string :secret
      t.datetime :confirmed_at
      t.bigint :last_totp_at
      t.timestamps
    end

    create_table :two_factor_backup_codes do |t|
      t.references :two_factor_credential, null: false, foreign_key: true
      t.string :code_digest, null: false
      t.datetime :used_at
      t.timestamps
    end
    add_index :two_factor_backup_codes, :code_digest, unique: true
  end
end
