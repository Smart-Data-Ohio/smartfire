class CreateBoardAutomationRuns < ActiveRecord::Migration[8.2]
  def change
    add_column :channel_threads, :work_status_changed_at, :datetime

    # Backfill the new column only: existing tracked work reads as if it
    # entered its status at its last update, so SLA timers start from a
    # real timestamp instead of NULL.
    reversible do |direction|
      direction.up do
        execute <<~SQL.squish
          UPDATE channel_threads
          SET work_status_changed_at = updated_at
          WHERE work_status IS NOT NULL AND work_status_changed_at IS NULL
        SQL
      end
    end

    create_table :board_sla_nudges do |t|
      t.references :room, null: false, foreign_key: true
      t.references :channel_thread, null: false, foreign_key: true
      t.string :work_status, null: false
      t.string :stage, null: false
      t.datetime :status_entered_at, null: false
      t.references :recipient, null: false, foreign_key: { to_table: :users }
      t.timestamps

      t.index [ :channel_thread_id, :work_status, :stage, :status_entered_at ],
        unique: true, name: "index_board_sla_nudges_on_claim"
    end

    create_table :board_stale_digests do |t|
      t.references :room, null: false, foreign_key: true
      t.date :digest_on, null: false
      t.references :message, foreign_key: true
      t.timestamps

      t.index [ :room_id, :digest_on ], unique: true
    end
  end
end
