class AddPerformanceIndexes < ActiveRecord::Migration[8.2]
  def change
    # Room pages (RoomsController#show) filter room_id + root thread and sort
    # by created_at; thread pages filter thread_id and sort the same way.
    add_index :messages, %i[room_id thread_id created_at], name: "index_messages_on_room_thread_created"
    add_index :messages, %i[thread_id created_at], name: "index_messages_on_thread_created"
    # The agent events poll filters agent_id + outcome and ranges/orders by id.
    add_index :agent_events, %i[agent_id outcome id], name: "index_agent_events_on_agent_outcome_id"
    # The reminder dispatcher filters reminded_at IS NULL + a starts_at range.
    add_index :events, %i[reminded_at starts_at], name: "index_events_on_reminded_starts"

    # The single-column messages indexes on room_id and thread_id are now
    # covered as leftmost prefixes by the composites above, but they stay:
    # migrations never drop pre-existing indexes.
  end
end
