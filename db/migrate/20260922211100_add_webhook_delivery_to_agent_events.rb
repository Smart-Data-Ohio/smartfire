class AddWebhookDeliveryToAgentEvents < ActiveRecord::Migration[8.2]
  def change
    add_column :agent_events, :webhook_status, :string, null: false, default: "none"
    add_column :agent_events, :webhook_attempts, :integer, null: false, default: 0
    add_column :agent_events, :webhook_last_error, :text
  end
end
