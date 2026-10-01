require "json"
require "active_support/testing/time_helpers"

# The nine declarations in test/models/huddle_revocation_test.rb, using real
# issue!/destroy!/ban/deactivate callbacks. Random participant identities are
# represented by their grant id in cleanup snapshots; their shape is checked.
class HuddleRevocationOracle
  include ActiveSupport::Testing::TimeHelpers

  CASES = {
    "membership" => ["Rooms::Closed", "membership"],
    "session" => ["Rooms::Closed", "session"],
    "ban" => ["Rooms::Closed", "ban"],
    "deactivate" => ["Rooms::Closed", "deactivate"],
    "room" => ["Rooms::Closed", "room"],
    "voice_membership" => ["Rooms::Voice", "membership"],
    "voice_room" => ["Rooms::Voice", "room"],
    "voice_deactivate" => ["Rooms::Voice", "deactivate"],
    "unavailable" => ["Rooms::Closed", "membership"]
  }

  def run
    ActiveRecord::Schema.verbose = false
    ENV["LIVEKIT_URL"] = "wss://huddle.example.test"
    ENV["LIVEKIT_INTERNAL_URL"] = "ws://livekit.example.test:7880"
    ENV["LIVEKIT_API_KEY"] = "ws13b-fixture-api-key"
    ENV["LIVEKIT_API_SECRET"] = "ws13b-fixture-api-secret"
    ENV["LIVEKIT_GATEWAY_SECRET"] = "ws13b-fixture-gateway-secret"
    ActionCable.server.define_singleton_method(:broadcast) { |*_| }
    Huddle::CleanupJob.define_singleton_method(:perform_later) { |*_| }
    Room::DestroyJob.define_singleton_method(:perform_later) { |*_| }
    cases = CASES.map { |name, (type, operation)| scenario(name, type, operation) }
    puts JSON.pretty_generate(reference_pin: "d7c7de92", now: Time.utc(2026, 1, 1, 12).to_i, cases: cases)
  ensure
    travel_back
  end

  def scenario(name, type, operation)
    travel_to Time.utc(2026, 1, 1, 12)
    load Rails.root.join("db/schema.rb")
    ActiveRecord::FixtureSet.reset_cache
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"), %w[accounts users rooms memberships])
    ENV["LIVEKIT_INTERNAL_URL"] = "ws://livekit.example.test:7880"
    david, jason = %w[david jason].map { |key| User.find(ActiveRecord::FixtureSet.identify(key)) }
    rooms = [9001, 9002].map { |id| Room.create!(id: id, type: type, name: "WS13b #{id}", creator: david) }
    members = rooms.each_with_index.flat_map do |room, index|
      [david, jason].each_with_index.map do |user, offset|
        room.memberships.create!(id: 9011 + index * 10 + offset, user: user, involvement: "everything")
      end
    end
    sessions = [david, david, jason].each_with_index.map do |user, index|
      Session.create!(id: 7001 + index, user: user, token: "ws13b-fixture-session-#{index}", user_agent: "WS13b")
    end
    input = {
      rooms: rooms.map(&:attributes), memberships: members.map(&:attributes), sessions: sessions.map(&:attributes),
      users: [david, jason].map { |u| u.attributes.slice("id", "name", "role", "status", "inbox_preferences") }
    }
    grants = []
    grants << HuddleGrant.issue!(session: sessions[0], membership: members[0])
    if %w[session deactivate].include?(name)
      grants << HuddleGrant.issue!(session: sessions[0], membership: members[2])
    elsif name == "ban"
      grants << HuddleGrant.issue!(session: sessions[1], membership: members[0])
    end
    # An unrelated user and another device must retain authorization.
    grants << HuddleGrant.issue!(session: sessions[2], membership: members[1])
    grants << HuddleGrant.issue!(session: sessions[1], membership: members[0]) unless name == "ban"
    travel 1.second
    grants.each { |g| g.update_columns(last_seen_at: Time.current) }
    before = snapshot
    ENV.delete("LIVEKIT_INTERNAL_URL") if name == "unavailable"
    case operation
    when "membership" then members[0].destroy!
    when "session" then sessions[0].destroy!
    when "ban" then david.ban
    when "deactivate" then david.deactivate
    when "room" then rooms[0].destroy!
    end
    { name: name, operation: operation, input: input, before: before, after: snapshot }
  end

  def snapshot
    grants = HuddleGrant.order(:id).map do |g|
      raise "bad identity" unless g.identity.match?(/\Acampfire-participant-[0-9a-f]{64}\z/)
      g.attributes.except("identity")
    end
    cleanups = HuddleCleanup.order(:id).map do |row|
      result = row.attributes
      result["identity"] = "grant:#{row.huddle_grant_id}" if row.identity
      result
    end
    { grants: grants, cleanups: cleanups }
  end
end

HuddleRevocationOracle.new.run
