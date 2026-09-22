require "digest"

# Bot keys ("<id>-<token>") were stored in plaintext. Keep the key value
# every existing bot URL carries, but store only its SHA-256 digest, as
# agent credentials already do. Plain SQL plus Ruby's digest: no models,
# jobs, Redis, or network.
class DigestBotTokens < ActiveRecord::Migration[8.2]
  def up
    add_column :users, :bot_token_digest, :string

    select_rows("SELECT id, bot_token FROM users WHERE bot_token IS NOT NULL AND bot_token != ''").each do |id, token|
      execute "UPDATE users SET bot_token_digest = #{quote(Digest::SHA256.hexdigest(token))} WHERE id = #{Integer(id)}"
    end

    add_index :users, :bot_token_digest, unique: true
    remove_index :users, :bot_token, name: "index_users_on_bot_token"
    remove_column :users, :bot_token
  end

  # The plaintext tokens are gone; rolling back restores the column empty,
  # so every bot needs a key reset afterwards.
  def down
    add_column :users, :bot_token, :string
    add_index :users, :bot_token, unique: true, name: "index_users_on_bot_token"
    remove_index :users, :bot_token_digest
    remove_column :users, :bot_token_digest
  end
end
