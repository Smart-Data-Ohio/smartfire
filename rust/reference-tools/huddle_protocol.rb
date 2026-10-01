require "json"
require "ostruct"
require "active_support/testing/time_helpers"

# Run in our pinned Rails image. No authorization header is persisted in the vectors.
class HuddleProtocolOracle
  include ActiveSupport::Testing::TimeHelpers

  def run
    ENV.update(
      "LIVEKIT_URL" => "wss://huddle.example.test",
      "LIVEKIT_INTERNAL_URL" => "ws://livekit.example.test:7880",
      "LIVEKIT_API_KEY" => "ws13-fixture-api-key",
      "LIVEKIT_API_SECRET" => "ws13-fixture-api-secret",
      "LIVEKIT_GATEWAY_SECRET" => "ws13-fixture-gateway-secret"
    )
    now = Time.utc(2026, 1, 1, 12)
    travel_to now
    original_uuid = SecureRandom.method(:uuid)
    SecureRandom.define_singleton_method(:uuid) { "00000000-0000-4000-8000-000000000013" }
    room_name = Huddle.room_name(42)
    identity = "campfire-participant-" + "13" * 32
    tokens = [
      [ "ordinary", false, nil, false ],
      [ "host", true, "host", false ],
      [ "speaker", true, "speaker", false ],
      [ "listener", true, "listener", false ],
      [ "missing_stage_role", true, nil, false ],
      [ "server_muted", false, nil, true ],
      [ "muted_host", true, "host", true ]
    ].map do |name, stage, role, muted|
      huddle = Huddle.allocate
      huddle.instance_variable_set(:@user, OpenStruct.new(name: "Fixture \"User\" ☃"))
      huddle.instance_variable_set(:@room, OpenStruct.new(stage?: stage))
      huddle.instance_variable_set(:@grant, OpenStruct.new(room_name: room_name, identity: identity, stage_role: role, server_muted?: muted))
      { name: name, stage: stage, role: role, muted: muted, token: huddle.token }
    end
    claims = JWT.decode(tokens.first.fetch(:token), nil, false).first
    shapes = []
    add = lambda do |name, modified, secret = ENV.fetch("LIVEKIT_API_SECRET"), algorithm = "HS256"|
      token = begin
        JWT.encode(modified, secret, algorithm)
      rescue JWT::InvalidPayload
        # The encoder rejects malformed time claims before the verifier can see them.
        input = [{ alg: algorithm }, modified].map { |v| Base64.urlsafe_encode64(JSON.generate(v), padding: false) }.join(".")
        "#{input}.#{Base64.urlsafe_encode64(OpenSSL::HMAC.digest('SHA256', secret, input), padding: false)}"
      end
      coordinates = begin
        Huddle::TokenVerifier.new(token).coordinates
      rescue Huddle::TokenVerifier::Invalid
        nil
      end
      shapes << { name: name, token: token, coordinates: coordinates }
    end
    mutate = -> { Marshal.load(Marshal.dump(claims)) }
    add.call("original", claims)
    add.call("wrong_secret", claims, "ws13-fixture-other-secret")
    add.call("wrong_algorithm", claims, ENV.fetch("LIVEKIT_API_SECRET"), "HS512")
    %w[ exp iss nbf sub video ].each { |key| add.call("missing_#{key}", claims.except(key)) }
    { "expired" => ["exp", now.to_i], "future" => ["nbf", now.to_i + 1],
      "float_exp" => ["exp", (now.to_i + 120).to_f], "string_nbf" => ["nbf", now.to_i.to_s],
      "wrong_issuer" => ["iss", "other"], "blank_sub" => ["sub", " \t"],
      "array_sub" => ["sub", [identity]], "null_video" => ["video", nil],
      "array_video" => ["video", []], "string_video" => ["video", "bad"] }.each do |name, (key, value)|
      add.call(name, claims.merge(key => value))
    end
    Huddle::TokenVerifier::FORBIDDEN_VIDEO_PERMISSIONS.each do |permission|
      [ true, 0, "", [], {} ].each_with_index do |value, index|
        c = mutate.call; c["video"][permission] = value
        add.call("forbidden_#{permission}_#{index}", c)
      end
    end
    Huddle::TokenVerifier::REQUIRED_VIDEO_PERMISSIONS.each do |permission|
      [ false, nil, 1, "true" ].each_with_index do |value, index|
        c = mutate.call; c["video"][permission] = value
        add.call("required_#{permission}_#{index}", c)
      end
    end
    [ [], ["camera"], Huddle::PUBLISH_SOURCES + ["camera"], Huddle::PUBLISH_SOURCES + ["unknown"],
      nil, "camera", [1], [{}] ].each_with_index do |value, index|
      c = mutate.call; c["video"]["canPublishSources"] = value
      add.call("bad_sources_#{index}", c)
    end
    c = mutate.call; c["video"]["unknown"] = false; add.call("unknown_false", c)
    c = mutate.call; c["video"]["room"] = " \t"; add.call("blank_room", c)
    c = mutate.call; c["video"]["canPublish"] = "yes"; add.call("string_publish", c)
    c = mutate.call; c["video"]["canPublish"] = false; add.call("listener_with_sources", c)
    c = mutate.call; c["video"]["canPublishSources"].reverse!; add.call("reordered_sources", c)
    c = mutate.call; c["video"].delete_if { |_, value| value == false }; add.call("refreshed_publisher", c)
    c = mutate.call; c["video"] = Huddle.participant_video_grant(room_name, publish: false).stringify_keys
    add.call("listener", c)
    c["video"].delete_if { |_, value| value == false || value == [] }; add.call("refreshed_listener", c)
    c["video"]["canPublish"] = nil; c["video"]["canPublishSources"] = nil; add.call("null_listener", c)

    urls = [
      ["wss://huddle.example.test", "ws://livekit.example.test:7880"],
      ["wss://SAME.test", "https://same.test:443"],
      ["ws://same.test/path", "http://same.test/other"],
      ["ws://same.test:81", "http://same.test:80"],
      ["https://same.test?different", "wss://same.test#fragment"],
      ["http://u:p@same.test", "ws://same.test"],
      ["wss://huddle.example.test", "ws://livekit.test/prefix/"],
      ["wss://huddle.example.test", "http://[::1]:7880/prefix//?ignored#fragment"],
      ["wss://huddle.example.test", "WSS://LIVEKIT.test/prefix"],
      ["ftp://huddle.test", "http://livekit.test"],
      ["/relative", "http://livekit.test"],
      ["wss://huddle.test", "http://bad host/"],
      ["wss://huddle.test", "http://livekit.test/ü"],
      ["wss://huddle.test", "ws://livekit.test:70000"],
      ["", "http://livekit.test"]
    ].map do |public_url, internal_url|
      ENV["LIVEKIT_URL"] = public_url; ENV["LIVEKIT_INTERNAL_URL"] = internal_url
      endpoint = begin
        Huddle::RoomService.new.send(:endpoint_uri, "RemoveParticipant").to_s
      rescue URI::InvalidURIError
        nil
      end
      { public_url: public_url, internal_url: internal_url, configured: Huddle.configured?, endpoint: endpoint }
    end
    result = {
      reference_pin: "d7c7de92", now: now.to_i, jti: SecureRandom.uuid,
      api_key: ENV.fetch("LIVEKIT_API_KEY"), api_secret: ENV.fetch("LIVEKIT_API_SECRET"),
      room_id: 42, room_name: room_name, identity: identity, tokens: tokens, shapes: shapes, urls: urls,
      admin_remove: Huddle::RoomService.new.send(:admin_token, { roomAdmin: true, room: room_name }),
      admin_delete: Huddle::RoomService.new.send(:admin_token, { roomCreate: true })
    }
    puts JSON.pretty_generate(result)
  ensure
    SecureRandom.define_singleton_method(:uuid, original_uuid) if original_uuid
    travel_back
  end
end
HuddleProtocolOracle.new.run
