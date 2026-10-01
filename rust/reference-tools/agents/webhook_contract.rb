# Run at d7c7de92 on a private default seed. Values come from our Rails models.
require "restricted_http/private_network_guard"
require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16) do
  hosts = %w[0.0.0.0 10.1.2.3 100.64.0.1 127.0.0.1 168.63.129.16 169.254.169.254 172.16.0.1 172.31.255.255 192.0.0.8 192.0.2.1 192.88.99.1 192.168.1.1 198.18.0.1 198.51.100.1 203.0.113.1 224.0.0.1 240.0.0.1 255.255.255.255 :: ::1 ::ffff:192.168.1.1 ::ffff:8.8.8.8 ::8.8.8.8 64:ff9b::a00:1 64:ff9b:1::808:808 ::ffff:0:a00:1 fc00::1 fd00::1 fe80::1 fec0::1 ff02::1 2001::1 2001:db8::1 2002::1 3fff::1 5f00::1 100::1 2001:2::1 4000::1 2001:10::1 2130706433 017700000001 0x7f000001 127.1 3232235521 [::1] [fd00::1] 8.8.8.8 1.1.1.1 93.184.216.34 172.32.0.1 100.128.0.1 192.0.1.1 2606:4700:4700::1111 2001:3::1 2001:4:112::1 64:ff9b::808:808 ::ffff:0:808:808 134744072 0x08080808 010.010.010.010 [2606:4700:4700::1111]]
  guards = hosts.map do |host|
    begin
      { host: host, address: RestrictedHTTP::PrivateNetworkGuard.resolve(host) }
    rescue => error
      { host: host, error: error.class.name }
    end
  end
  original = Resolv.method(:getaddresses)
  dns = [ ["private.example", ["10.0.0.1"]], ["mixed.example", ["10.0.0.1", "2606:4700:4700::1111", "93.184.216.34"]], ["empty.example", []] ].map do |host, answers|
    Resolv.define_singleton_method(:getaddresses) { |_host| answers }
    begin
      { host: host, answers: answers, address: RestrictedHTTP::PrivateNetworkGuard.resolve(host) }
    rescue => error
      { host: host, answers: answers, error: error.class.name }
    end
  end
  Resolv.define_singleton_method(:getaddresses, original)
  bodies = ["{}", %({"body":"<b>Hi</b>&bye"}), "é😀", "line\nend"]
  signatures = bodies.map do |body|
    timestamp = Time.current.to_i.to_s
    secret = "ws11-public-test-signing-secret"
    { body: body, secret: secret, timestamp: timestamp, signature: "sha256=#{OpenSSL::HMAC.hexdigest('SHA256', secret, "#{timestamp}.#{body}")}" }
  end
  bot = User.find(394959859)
  webhook = bot.webhook
  message = Message.find(136976342)
  # Use an existing message rather than creating one: stable ids and stored body bytes.
  agent = bot.agent
  payloads = { agent_backed_legacy_delivery: webhook.send(:payload, message), agent_delivery: webhook.send(:payload, message, agent: agent, delivery_id: 123) }
  agent.destroy!
  payloads[:legacy_delivery] = webhook.send(:payload, message)
  secret = webhook.ensure_signing_secret!
  raw = webhook.reload.read_attribute_before_type_cast(:signing_secret)
  puts JSON.pretty_generate(reference_pin: "d7c7de92", now: Time.current.iso8601, guards: guards, dns: dns, signatures: signatures,
    payloads: payloads, secret: { plaintext: secret, ciphertext: raw, encoding: secret.encoding.to_s, repeated: webhook.ensure_signing_secret! == secret })
end
