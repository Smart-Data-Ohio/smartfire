class AddDndUntilToUsers < ActiveRecord::Migration[8.2]
  def change
    add_column :users, :dnd_until, :datetime
  end
end
