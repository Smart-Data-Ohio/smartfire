class CreateWorkHandoffs < ActiveRecord::Migration[8.2]
  def change
    create_table :work_handoffs do |t|
      t.references :channel_thread, null: false, foreign_key: true
      t.references :sender, null: false, foreign_key: { to_table: :users }
      t.references :receiver_agent, null: false, foreign_key: { to_table: :agents }
      t.text :summary, null: false
      t.json :links, null: false, default: []
      t.json :open_questions, null: false, default: []
      t.timestamps

      t.index [ :channel_thread_id, :created_at ]
    end
  end
end
