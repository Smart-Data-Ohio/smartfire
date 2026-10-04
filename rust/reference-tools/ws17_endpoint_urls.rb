# Exercise the model's real validation with the network guard returning a public IP.
ActiveRecord::Base.logger = nil
user = User.find_by!(email_address: "david@37signals.com")
guard = RestrictedHTTP::PrivateNetworkGuard.singleton_class
original = RestrictedHTTP::PrivateNetworkGuard.method(:resolve)
guard.define_method(:resolve) { |host| "142.250.185.206" }
endpoints = [nil, "", " ", "https://fcm.googleapis.com/%zz", "https://fcm.googleapis.com/{invalid}", "https://fcm.googleapis.com/é", "https://fcm.googleapis.com/%C3%A9", "https://fcm.googleapis.com/%", "https://fcm.googleapis.com/%a", "https://fcm.googleapis.com/?q=%zz", "https://fcm.googleapis.com/?q=%z1", "https://fcm.googleapis.com/?q=%1z", "https://fcm.googleapis.com/?q=%", "https://fcm.googleapis.com/?q={invalid}", "https://fcm.googleapis.com/#%zz", "https://fcm.googleapis.com/#é", "HTTPS://FCM.GOOGLEAPIS.COM/path", "https://fcm.googleapis.com:", "https://fcm.googleapis.com:00443/path", "https://fcm.googleapis.com:65536/path", "https://fcm.googleapis.com:9999999999999999999999999/path", "https://fcm.googleapis.com:abc/path", "https://fcm.googleapis.com:443:443/path", "https://fcm.googleapis.com./path", "https://sub.fcm.googleapis.com/path", "https://fcm.googleapis.com.evil.test/path", "https://evil.test@fcm.googleapis.com/path", "https://x@y@fcm.googleapis.com/path", "https://[::1]/path", "https://[invalid]/path", "https://[v1.host]/path", "https:///path", "https:path", "https:/path", "//fcm.googleapis.com/path", "/path", "path", "http://fcm.googleapis.com/path", "mailto:user@example.com", "1https://fcm.googleapis.com/path", "https://fcm.googleapis.com\\path", "https://fcm.googleapis.com/path#fragment#second"]
(0..127).each do |point|
  char = point.chr
  endpoints.concat(["https://fcm.googleapis.com/a#{char}b", "https://fcm.googleapis.com/?a=#{char}b", "https://fcm.googleapis.com/#a#{char}b", "https://a#{char}b.fcm.googleapis.com/path", "https://a#{char}b@fcm.googleapis.com/path"])
end
rows = endpoints.uniq.map do |endpoint|
  sub = user.push_subscriptions.new(endpoint:, p256dh_key: "test_key", auth_key: "test_auth")
  {endpoint:, valid: sub.valid?, errors: sub.errors.full_messages, resolved: sub.resolved_endpoint_ip}
end
puts JSON.generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], uri: Gem.loaded_specs.fetch("uri").version.to_s, rows:)
guard.define_method(:resolve, original)
