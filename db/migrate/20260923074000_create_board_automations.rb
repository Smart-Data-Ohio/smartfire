class CreateBoardAutomations < ActiveRecord::Migration[8.2]
  def change
    create_table :board_tag_assignments do |t|
      t.references :room, null: false, foreign_key: true
      t.string :tag, null: false
      t.references :assignee, null: false, foreign_key: { to_table: :users }
      t.references :created_by, null: false, foreign_key: { to_table: :users }
      t.timestamps

      t.index [ :room_id, :tag ], unique: true
    end

    create_table :board_sla_rules do |t|
      t.references :room, null: false, foreign_key: true
      t.string :work_status, null: false
      t.integer :nudge_after_minutes, null: false
      t.integer :escalate_after_minutes, null: false
      t.timestamps

      t.index [ :room_id, :work_status ], unique: true
    end
  end
end
