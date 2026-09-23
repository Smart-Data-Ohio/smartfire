class AddInboundEmailTokenToRooms < ActiveRecord::Migration[8.2]
  def change
    add_column :rooms, :inbound_email_token, :string
    add_index :rooms, :inbound_email_token, unique: true
  end
end
