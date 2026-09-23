class AddBoardPostOpenerToMessages < ActiveRecord::Migration[8.2]
  # Marks board-post opening messages so the agent message budget can skip
  # them: an opener counts only toward the board-post cap. Old openers keep
  # their default and age out of today's usage window within a day.
  def change
    add_column :messages, :board_post_opener, :boolean, default: false, null: false
  end
end
