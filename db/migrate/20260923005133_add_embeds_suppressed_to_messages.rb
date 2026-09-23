class AddEmbedsSuppressedToMessages < ActiveRecord::Migration[8.2]
  def change
    add_column :messages, :embeds_suppressed, :boolean, default: false, null: false
  end
end
