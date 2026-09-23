class AddAgentPresenceAndBudgets < ActiveRecord::Migration[8.2]
  def change
    add_column :agents, :working_presence, :string
    add_column :agents, :working_presence_expires_at, :datetime
    add_column :agents, :daily_message_cap, :integer
    add_column :agents, :daily_board_post_cap, :integer
    add_column :agents, :daily_external_action_cap, :integer

    create_table :agent_budget_notices do |t|
      t.integer :agent_id, null: false
      t.string :cap, null: false
      t.date :day, null: false

      t.timestamps
    end

    add_index :agent_budget_notices, [ :agent_id, :cap, :day ],
      unique: true, name: "index_agent_budget_notices_on_agent_cap_day"
  end
end
