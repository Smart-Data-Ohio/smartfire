class AddMessageIdempotencyIndex < ActiveRecord::Migration[8.2]
  def change
    # Backs Message.find_duplicate: a retried create looks up the original
    # by room + author + client id.
    add_index :messages, %i[room_id creator_id client_message_id], name: "index_messages_on_room_creator_client_id"
  end
end
