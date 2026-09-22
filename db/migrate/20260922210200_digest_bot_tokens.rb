require "digest"

# Bot keys ("<id>-<token>") are stored in plaintext. This release adds the
# token's SHA-256 digest, as agent credentials store theirs, and the app
# authenticates against it. The plaintext column stays populated this
# release so a rolled-back container still authenticates and legacy webhook
# payloads keep their working room.path; a follow-up release drops it.
# Plain SQL plus Ruby's digest: no models, jobs, Redis, or network.
class DigestBotTokens < ActiveRecord::Migration[8.2]
  def up
    add_column :users, :bot_token_digest, :string

    select_rows("SELECT id, bot_token FROM users WHERE bot_token IS NOT NULL AND bot_token != ''").each do |id, token|
      execute "UPDATE users SET bot_token_digest = #{quote(Digest::SHA256.hexdigest(token))} WHERE id = #{Integer(id)}"
    end

    add_index :users, :bot_token_digest, unique: true
  end

  # Lossless: the plaintext column was never touched.
  # Rolls back with SQLite's native DROP COLUMN. remove_column would rebuild
  # the table inside the migration transaction, where foreign keys cannot be
  # switched off, so dropping the old table would fire ON DELETE actions on
  # the tables that reference it.
  def down
    remove_index :users, :bot_token_digest
    execute "ALTER TABLE users DROP COLUMN bot_token_digest"
  end
end
