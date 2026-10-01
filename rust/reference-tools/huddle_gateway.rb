require "json"
require "active_support/testing/time_helpers"

class HuddleGatewayOracle
  include ActiveSupport::Testing::TimeHelpers
  def run
    ActiveRecord::Schema.verbose = false
    load Rails.root.join("db/schema.rb")
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"), %w[accounts users rooms memberships])
    travel_to Time.utc(2026, 1, 1, 12)
    key = "ws13-fixture-api-key"; secret = "ws13-fixture-api-secret"; gateway = "ws13-fixture-gateway-secret"
    ENV.update("LIVEKIT_URL" => "wss://public.example.test", "LIVEKIT_INTERNAL_URL" => "http://internal.example.test:7880", "LIVEKIT_API_KEY" => key, "LIVEKIT_API_SECRET" => secret, "LIVEKIT_GATEWAY_SECRET" => gateway)
    user = User.find(ActiveRecord::FixtureSet.identify(:david))
    room = Room.find(ActiveRecord::FixtureSet.identify(:watercooler))
    membership = room.memberships.find_by!(user: user)
    session = user.sessions.create!(user_agent: "ws13")
    membership_attributes = membership.attributes
    jobs = []
    Huddle::BroadcastPresenceJob.define_singleton_method(:perform_later) { |id| jobs << "presence" }
    Huddle::JoinNoticeJob.define_singleton_method(:perform_later) { |id| jobs << "join" }
    # Capture only external effects: the request/controller/model paths stay real.
    Huddle::CleanupJob.define_singleton_method(:perform_later) { |id| jobs << "cleanup" }
    claims = {exp: Time.current.to_i + 120, iat: Time.current.to_i, iss: key, jti: "ws13-fixture-jti", name: user.name, nbf: Time.current.to_i-5, sub: "ws13-security-participant", video: Huddle.participant_video_grant("ws13-security-room")}
    token = JWT.encode(claims, secret, "HS256")
    expired = JWT.encode(claims.merge(exp: Time.current.to_i), secret, "HS256")
    cases = [
      {name: "missing_secret", secret: nil}, {name: "wrong_secret", secret: "wrong"}, {name: "blank_secret", secret: " "},
      {name: "valid"}, {name: "missing_bearer", bearer: nil}, {name: "wrong_scheme", scheme: "Basic"}, {name: "case_insensitive_scheme", scheme: "bEaReR"},
      {name: "malformed_token", bearer: "malformed"}, {name: "expired_token", bearer: expired}, {name: "revoked", revoked: true}, {name: "removed_member", removed: true},
      {name: "unconfigured", configured: false},
      {name: "show", method: "get", path: "/internal/huddle/grants/17"},
      {name: "show_no_sighting", method: "get", path: "/internal/huddle/grants/17?record_seen=0"},
      {name: "show_query_zero", method: "get", path: "/internal/huddle/grants/17", params: {record_seen: 0}},
      {name: "show_revoked", method: "get", path: "/internal/huddle/grants/17", revoked: true},
      {name: "show_missing", method: "get", path: "/internal/huddle/grants/999999"},
      {name: "show_id_suffix", method: "get", path: "/internal/huddle/grants/17tail"},
      {name: "show_id_nonnumeric", method: "get", path: "/internal/huddle/grants/nope"},
      {name: "left", path: "/internal/huddle/grants/17/left", seen: true},
      {name: "left_missing", path: "/internal/huddle/grants/999999/left"},
      {name: "left_revoked", path: "/internal/huddle/grants/17/left", seen: true, revoked: true},
    ]
    [nil, "", " ", false, 0, 42, [], {}, "garbage", "2026-01-01T12:00:00Z", "2026-01-01T07:00:00-05:00", "2026-01-01", "12:00", "2026-01-01T11:59:59Z", "2026-02-30", "999999999-01-01", "2026-01-01T12:00:00.000001Z"].each_with_index do |raw,i|
      cases << {name: "left_timestamp_#{i}", path: "/internal/huddle/grants/17/left", seen: true, params: {disconnected_at: raw}}
    end
    results = []
    cases.each do |spec|
      begin
        HuddleCleanup.delete_all
        HuddleGrant.delete_all
        Membership.upsert_all([membership_attributes])
        ENV["LIVEKIT_URL"] = spec[:configured] == false ? "" : "wss://public.example.test"
        grant = HuddleGrant.create!(id: 17, identity: claims[:sub], room_name: claims[:video][:room], session: session, user: user, membership: membership, room: room)
        grant.update_columns(last_seen_at: Time.current) if spec[:seen]
        grant.update_columns(revoked_at: Time.current) if spec[:revoked]
        Membership.where(id: membership.id).delete_all if spec[:removed]
        provided = spec.key?(:secret) ? spec[:secret] : gateway
        bearer = spec.key?(:bearer) ? spec[:bearer] : token
        headers = {"HTTP_X_FORWARDED_PROTO" => "https", "ACCEPT" => "application/json", "CONTENT_TYPE" => "application/json"}
        headers["X-Huddle-Gateway-Secret"] = provided unless provided.nil?
        headers["Authorization"] = "#{spec[:scheme] || 'Bearer'} #{bearer}" unless bearer.nil?
        request = ActionDispatch::Integration::Session.new(Rails.application)
        jobs.clear
        request.public_send(spec[:method] || "post", spec[:path] || "/internal/huddle/authorize", params: spec[:method] == "get" ? spec[:params] : JSON.generate(spec[:params] || {}), headers: headers)
        result = spec.merge(status: request.response.status, cache: request.response.headers["Cache-Control"], body: request.response.body.empty? ? nil : JSON.parse(request.response.body), seen_after: grant.reload.last_seen_at&.to_i, jobs: jobs.dup)
      ensure
        results << result if result
      end
    end
    puts JSON.pretty_generate({reference_pin: "d7c7de92", now: Time.current.to_i, token: token, cases: results.compact})
  ensure
    travel_back
  end
end
HuddleGatewayOracle.new.run
