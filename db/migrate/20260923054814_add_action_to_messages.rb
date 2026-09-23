class AddActionToMessages < ActiveRecord::Migration[8.2]
  def change
    add_column :messages, :action, :boolean, null: false, default: false
  end
end
