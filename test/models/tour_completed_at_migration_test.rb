require "test_helper"
require_relative "../../db/migrate/20260923053807_add_tour_completed_at_to_users"

# Runs the tour-completed-at migration against a scratch in-memory SQLite
# database, so the backfill SQL is exercised exactly as production runs it.
class TourCompletedAtMigrationTest < ActiveSupport::TestCase
  class ScratchRecord < ActiveRecord::Base
    self.abstract_class = true
  end

  setup do
    ScratchRecord.establish_connection(adapter: "sqlite3", database: ":memory:")
    @connection = ScratchRecord.connection

    @connection.create_table "users", force: :cascade do |t|
      t.string "name", null: false
    end

    @connection.execute "INSERT INTO users (name) VALUES ('Existing One'), ('Existing Two')"
  end

  teardown do
    ScratchRecord.remove_connection
  end

  test "up stamps existing users and leaves the column nullable for new members" do
    migrate(:up)

    assert_equal 2, @connection.select_value("SELECT COUNT(*) FROM users")
    assert_equal 2, @connection.select_value("SELECT COUNT(*) FROM users WHERE tour_completed_at IS NOT NULL")

    @connection.execute "INSERT INTO users (name) VALUES ('New Member')"
    assert_nil @connection.select_value("SELECT tour_completed_at FROM users WHERE name = 'New Member'")
  end

  test "down removes only the tour column" do
    before_columns = @connection.columns(:users).map(&:name)

    migrate(:up)
    migrate(:down)

    assert_equal before_columns, @connection.columns(:users).map(&:name)
    assert_equal 2, @connection.select_value("SELECT COUNT(*) FROM users")
  end

  private
    def migrate(direction)
      ActiveRecord::Migration.suppress_messages do
        AddTourCompletedAtToUsers.new.exec_migration(@connection, direction)
      end
    end
end
