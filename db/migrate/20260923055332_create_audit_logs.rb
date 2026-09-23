class CreateAuditLogs < ActiveRecord::Migration[8.2]
  def change
    create_table :audit_logs do |t|
      t.string :action, null: false
      # Plain integer columns, deliberately no foreign keys: audit rows
      # outlive the users and targets they describe (rooms are deleted,
      # users deactivated), and the log itself is append-only.
      t.bigint :actor_id
      t.string :actor_label
      t.string :target_type
      t.bigint :target_id
      t.string :target_label
      t.json :details
      t.string :ip_address
      t.string :user_agent

      t.timestamps
    end

    add_index :audit_logs, :action
    add_index :audit_logs, :actor_id
    add_index :audit_logs, [ :target_type, :target_id ]
    add_index :audit_logs, :created_at
  end
end
