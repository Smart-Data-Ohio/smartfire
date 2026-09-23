class AddActionIpCreatedAtIndexToAuditLogs < ActiveRecord::Migration[8.2]
  def change
    add_index :audit_logs, [ :action, :ip_address, :created_at ]
  end
end
