class AddTourCompletedAtToUsers < ActiveRecord::Migration[8.2]
  def change
    add_column :users, :tour_completed_at, :datetime
  end
end
