class AddSystemNoteToMessages < ActiveRecord::Migration[8.2]
  def change
    # True for quiet timeline notes (pin notes): rendered in the timeline
    # like any message, but never marking the room unread, pushing,
    # delivering to agents, recording inbox items, or indexing for search.
    add_column :messages, :system_note, :boolean, default: false, null: false
  end
end
