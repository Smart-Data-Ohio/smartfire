class AddMessageProcessingLeaseToActiveStorageBlobs < ActiveRecord::Migration[8.2]
  def change
    # Keep scheduling state outside metadata: concurrent Active Storage
    # analysis replaces metadata and could otherwise erase a pending claim.
    add_column :active_storage_blobs, :message_processing_token, :string
    add_column :active_storage_blobs, :message_processing_expires_at, :datetime
  end
end
