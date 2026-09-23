class CreateAgentSteps < ActiveRecord::Migration[8.2]
  def change
    create_table :agent_steps do |t|
      t.integer :agent_id, null: false
      t.integer :message_id
      t.integer :channel_thread_id
      t.string :name, null: false
      t.string :status, default: "running", null: false
      t.text :input_summary
      t.text :output_summary
      t.integer :duration_ms
      t.integer :position, default: 0, null: false

      t.timestamps
    end

    add_index :agent_steps, :agent_id
    add_index :agent_steps, :message_id
    add_index :agent_steps, :channel_thread_id
  end
end
