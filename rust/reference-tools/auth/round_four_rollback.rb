user = User.find(127326141)
valid = user.valid?
puts "WS9 zone readback: zone=#{user.time_zone.inspect}; explicit=#{user.time_zone_explicit.inspect}; valid=#{valid}; errors=#{user.errors.to_hash.inspect}"
raise "ROLLBACK FAILURE: Rails rejected Rust's time zone" unless valid
raise "wrong-case request changed the saved zone" unless user.time_zone == "UTC"
raise "rejected request changed the explicit marker" if user.time_zone_explicit?
raise "zone-only request invented an audit" if AuditLog.exists?
puts "WS9 zone rollback: Rails validated Rust's unchanged UTC row; rejected wrong-case zone wrote no field, explicit marker or audit"
