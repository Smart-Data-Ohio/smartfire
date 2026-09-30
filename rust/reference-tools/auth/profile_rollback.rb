# Reads the DB exported after Rust's actual authenticated PATCH (not a model-only fixture).
user = User.find(127326141)
raise "wrong Rust profile email" unless user.email_address == "ws9-reviewed@example.test"
raise "preexisting Google identity" if user.google_identity
claims = { "sub" => "ws9-reviewed-new-subject", "email" => user.email_address, "hd" => "example.test", "email_verified" => true }
begin
  linked = Google::SignIn::AccountLinker.resolve!(claims)
  raise "ROLLBACK FAILURE: Rails auto-linked the new Google subject to Rust user #{linked.id}; marker=#{user.reload.email_self_changed_at.inspect}"
rescue Google::SignIn::Rejected => error
  raise "wrong rejection: #{error.message}" unless error.message.include?("admin_link_required")
end
raise "missing self-change marker" unless user.reload.email_self_changed_at.present?
raise "Rust incorrectly changed Google's allow flag" unless user.google_email_link_allowed?
raise "Rails validation rejected Rust user: #{user.errors.full_messages}" unless user.valid?
row = AuditLog.find_by!(action: "user.email.change", target_id: user.id)
expected = { "email_address" => { "before" => "david@37signals.com", "after" => user.email_address } }
raise "wrong audit details" unless row.details == expected
raise "wrong audit identity" unless row.actor_id == user.id && row.target_type == "User" && row.actor_label == AuditLog.label_for(user) && row.target_label == AuditLog.label_for(user)
raise "identity was saved despite rejection" if GoogleIdentity.exists?(subject: claims["sub"])
puts "WS9 profile rollback: Rails rejected new Google subject with admin_link_required; self-change marker and email audit validated; no identity created"
