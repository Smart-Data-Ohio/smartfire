require "test_helper"
require_relative "../../db/migrate/20260925174854_add_messages_count_to_channel_threads"

# Runs the thread reply counter migration against a scratch in-memory SQLite
# database, so the backfill SQL is exercised exactly as production runs it.
class MessagesCountMigrationTest < ActiveSupport::TestCase
  class ScratchRecord < ActiveRecord::Base
    self.abstract_class = true
  end

  setup do
    ScratchRecord.establish_connection(adapter: "sqlite3", database: ":memory:")
    @connection = ScratchRecord.connection

    @connection.create_table "channel_threads", force: :cascade do |t|
      t.string "name", null: false
    end
    @connection.create_table "messages", force: :cascade do |t|
      t.integer "thread_id"
      t.boolean "system_note", default: false, null: false
      t.boolean "streaming", default: false, null: false
    end

    # Ids straddle the backfill's batch edges.
    batch = AddMessagesCountToChannelThreads::BATCH_SIZE
    @ids = [ 1, batch, batch + 1, batch * 2 + 5 ]
    @ids.each { |id| @connection.execute "INSERT INTO channel_threads (id, name) VALUES (#{id}, 'Thread #{id}')" }

    insert_messages(@ids[0], 3)
    insert_messages(@ids[0], 1, system_note: true)
    insert_messages(@ids[0], 1, streaming: true)
    insert_messages(@ids[1], 1)
    insert_messages(@ids[3], 2)
    insert_messages(nil, 4)
  end

  teardown do
    ScratchRecord.remove_connection
  end

  test "up backfills finished, non-system replies across batches" do
    migrate(:up)

    counts = @connection.select_rows("SELECT id, messages_count FROM channel_threads ORDER BY id").to_h
    assert_equal({ @ids[0] => 3, @ids[1] => 1, @ids[2] => 0, @ids[3] => 2 }, counts)
  end

  test "the backfill matches the app's own recount" do
    migrate(:up)
    backfilled = @connection.select_rows("SELECT id, messages_count FROM channel_threads ORDER BY id")

    @connection.execute "UPDATE channel_threads SET #{ChannelThread::REPLY_COUNT_SQL}"

    assert_equal backfilled, @connection.select_rows("SELECT id, messages_count FROM channel_threads ORDER BY id")
  end

  test "new threads default to zero" do
    migrate(:up)

    @connection.execute "INSERT INTO channel_threads (name) VALUES ('Fresh')"
    assert_equal 0, @connection.select_value("SELECT messages_count FROM channel_threads WHERE name = 'Fresh'")
  end

  test "down removes only the counter column" do
    before_columns = @connection.columns(:channel_threads).map(&:name)

    migrate(:up)
    migrate(:down)

    assert_equal before_columns, @connection.columns(:channel_threads).map(&:name)
    assert_equal 4, @connection.select_value("SELECT COUNT(*) FROM channel_threads")
  end

  private
    def insert_messages(thread_id, count, system_note: false, streaming: false)
      count.times do
        @connection.execute "INSERT INTO messages (thread_id, system_note, streaming) VALUES " \
          "(#{thread_id || "NULL"}, #{system_note ? 1 : 0}, #{streaming ? 1 : 0})"
      end
    end

    def migrate(direction)
      ActiveRecord::Migration.suppress_messages do
        AddMessagesCountToChannelThreads.new.exec_migration(@connection, direction)
      end
    end
end
