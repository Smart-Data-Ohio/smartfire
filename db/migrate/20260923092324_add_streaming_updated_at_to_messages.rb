class AddStreamingUpdatedAtToMessages < ActiveRecord::Migration[8.2]
  def change
    add_column :messages, :streaming_updated_at, :datetime
    add_index :messages, :streaming_updated_at,
      name: "index_messages_on_streaming_updated_at", where: "streaming = 1"

    # Existing open streams inherit their creation time as their last
    # activity; the sweep backfills the same for any rows the old code
    # writes mid-deploy. Only the new column is written.
    reversible do |direction|
      direction.up do
        execute "UPDATE messages SET streaming_updated_at = created_at WHERE streaming = 1 AND streaming_updated_at IS NULL"
      end
    end
  end
end
