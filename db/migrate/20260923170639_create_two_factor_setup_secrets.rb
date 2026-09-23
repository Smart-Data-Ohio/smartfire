class CreateTwoFactorSetupSecrets < ActiveRecord::Migration[8.2]
  # Unconfirmed TOTP secrets for the setup page, one row per session at
  # most. Each visit to setup issues a fresh secret bound to the current
  # session, so someone who opens setup cannot learn the secret another
  # session later confirms. The secret is encrypted at rest by Active
  # Record encryption (see TwoFactorSetupSecret); expired rows are pruned
  # by the retention job.
  def change
    create_table :two_factor_setup_secrets do |t|
      t.references :session, null: false, index: { unique: true }, foreign_key: { on_delete: :cascade }
      t.string :secret
      t.datetime :expires_at, null: false
      t.timestamps
    end
  end
end
