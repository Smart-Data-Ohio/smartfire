class AddTextSizeToUsers < ActiveRecord::Migration[8.1]
  def change
    add_column :users, :text_size, :string, null: false, default: "default"
  end
end
