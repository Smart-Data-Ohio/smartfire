class AddStreamingToMessages < ActiveRecord::Migration[8.2]
  def change
    add_column :messages, :streaming, :boolean, default: false, null: false
    add_column :messages, :stream_broadcast_at, :datetime
    add_index :messages, [ :streaming, :created_at ],
      name: "index_messages_on_streaming_and_created_at", where: "streaming"
  end
end
