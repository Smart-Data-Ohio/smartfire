require "test_helper"
require_relative "../../db/migrate/20260922210200_digest_bot_tokens"

# Runs the bot-token digest migration against a scratch in-memory SQLite
# database whose users table is origin/main's (before 20260922210000), so
# the backfill SQL is exercised exactly as production runs it.
class DigestBotTokensMigrationTest < ActiveSupport::TestCase
  class ScratchRecord < ActiveRecord::Base
    self.abstract_class = true
  end

  setup do
    ScratchRecord.establish_connection(adapter: "sqlite3", database: ":memory:")
    @connection = ScratchRecord.connection

    # origin/main's users table, verbatim from its db/schema.rb.
    @connection.create_table "users", force: :cascade do |t|
      t.text "bio"
      t.string "bot_token"
      t.datetime "created_at", null: false
      t.string "email_address"
      t.string "github_login"
      t.string "icon_name"
      t.json "inbox_preferences", default: {}
      t.string "name", null: false
      t.string "password_digest"
      t.integer "role", default: 0, null: false
      t.integer "status", default: 0, null: false
      t.datetime "updated_at", null: false
      t.index "LOWER(github_login)", name: "index_users_on_lower_github_login", unique: true, where: "github_login IS NOT NULL"
      t.index [ "bot_token" ], name: "index_users_on_bot_token", unique: true
      t.index [ "email_address" ], name: "index_users_on_email_address", unique: true
    end

    insert_user("Bot One", role: 2, bot_token: "BotOneToken1")
    insert_user("Quote Bot", role: 2, bot_token: "x'y--z")
    insert_user("Human", role: 0, bot_token: nil, email_address: "human@smartdata.net")
    insert_user("Blank Bot", role: 2, bot_token: "")
  end

  teardown do
    ScratchRecord.remove_connection
  end

  test "up backfills the digest of every stored token and leaves the plaintext untouched" do
    before = rows

    migrate(:up)

    after = rows
    assert_equal before.map { |row| row.except("bot_token_digest") }, after.map { |row| row.except("bot_token_digest") }
    assert_equal Digest::SHA256.hexdigest("BotOneToken1"), digest_for("Bot One")
    assert_equal Digest::SHA256.hexdigest("x'y--z"), digest_for("Quote Bot")
    assert_nil digest_for("Human")
    assert_nil digest_for("Blank Bot")
    assert @connection.index_exists?(:users, :bot_token_digest, unique: true)
    assert @connection.index_exists?(:users, :bot_token, unique: true)
  end

  test "down removes only the digest, restoring the original table" do
    before_columns = @connection.columns(:users).map(&:name)
    before = rows

    migrate(:up)
    migrate(:down)

    assert_equal before_columns, @connection.columns(:users).map(&:name)
    assert_equal before, rows
  end

  private
    def insert_user(name, role:, bot_token:, email_address: nil)
      @connection.execute <<~SQL
        INSERT INTO users (name, role, status, bot_token, email_address, created_at, updated_at)
        VALUES (#{@connection.quote(name)}, #{role}, 0, #{@connection.quote(bot_token)}, #{@connection.quote(email_address)}, '2026-09-01 00:00:00', '2026-09-01 00:00:00')
      SQL
    end

    def migrate(direction)
      ActiveRecord::Migration.suppress_messages do
        DigestBotTokens.new.exec_migration(@connection, direction)
      end
    end

    def rows
      @connection.select_all("SELECT * FROM users ORDER BY id").to_a
    end

    def digest_for(name)
      @connection.select_value("SELECT bot_token_digest FROM users WHERE name = #{@connection.quote(name)}")
    end
end
