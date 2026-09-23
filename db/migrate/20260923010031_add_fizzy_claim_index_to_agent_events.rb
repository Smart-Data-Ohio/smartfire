class AddFizzyClaimIndexToAgentEvents < ActiveRecord::Migration[8.2]
  def change
    add_index :agent_events, %i[ agent_id agent_approval_id ],
      name: "index_agent_events_on_agent_fizzy_approval",
      unique: true,
      where: "event_type = 'fizzy_action_completed' AND agent_approval_id IS NOT NULL"
  end
end
