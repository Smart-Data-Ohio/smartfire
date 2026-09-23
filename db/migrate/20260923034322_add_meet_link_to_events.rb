class AddMeetLinkToEvents < ActiveRecord::Migration[8.2]
  def change
    add_column :events, :meet_link, :string
    add_column :events, :meet_link_requested, :boolean, null: false, default: false
  end
end
