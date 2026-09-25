# Counter behind the room's "N replies" thread indicator: the thread's
# finished, non-system messages (see ChannelThread::REPLY_COUNT_SQL, which
# this backfill mirrors). The app keeps it current by recomputing from the
# rows, so a count written by old code mid-deploy heals on the next reply.
# Plain SQL on the new column only, in id-range batches: no models,
# callbacks, jobs, broadcasts, Redis, or network.
class AddMessagesCountToChannelThreads < ActiveRecord::Migration[8.2]
  BATCH_SIZE = 1_000

  def up
    add_column :channel_threads, :messages_count, :integer, default: 0, null: false

    max_id = select_value("SELECT MAX(id) FROM channel_threads").to_i
    (0...max_id).step(BATCH_SIZE) do |floor|
      execute <<~SQL.squish
        UPDATE channel_threads
           SET messages_count = (
             SELECT COUNT(*) FROM messages
              WHERE messages.thread_id = channel_threads.id
                AND messages.system_note = 0
                AND messages.streaming = 0
           )
         WHERE id > #{floor} AND id <= #{floor + BATCH_SIZE}
      SQL
    end
  end

  # Lossless: the column never existed before this release. Rolls back
  # with SQLite's native DROP COLUMN: remove_column would rebuild the table
  # inside the migration transaction and fire ON DELETE actions on the
  # tables that reference channel_threads.
  def down
    execute "ALTER TABLE channel_threads DROP COLUMN messages_count"
  end
end
