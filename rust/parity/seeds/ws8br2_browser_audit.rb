# test/system/audit_log_test.rb: build both rows through the pinned Rails producer.
based_on "default"
at NOW
Current.reset
AuditLog.delete_all
AuditLog.record!(action: "user.ban", actor: user(:david), target: user(:kevin),
  changes: { status: %w[ active banned ] })
AuditLog.record!(action: "room.create", actor: user(:david), target: room(:watercooler),
  changes: { name: "All Talk" })
Time.use_zone(user(:david).time_zone.presence || "UTC") do
  label :browser, :audit_csv, Accounts::AuditLogsController.new.send(:audit_csv, AuditLog.where(action: "user.ban"))
end
