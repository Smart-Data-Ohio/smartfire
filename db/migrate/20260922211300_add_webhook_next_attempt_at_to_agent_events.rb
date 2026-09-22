class AddWebhookNextAttemptAtToAgentEvents < ActiveRecord::Migration[8.2]
  def change
    add_column :agent_events, :webhook_next_attempt_at, :datetime
    add_index :agent_events, [ :webhook_status, :webhook_next_attempt_at ],
      name: "index_agent_events_on_webhook_recovery"
  end
end
