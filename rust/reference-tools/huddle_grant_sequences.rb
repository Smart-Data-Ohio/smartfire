require "json"
require "active_support/testing/time_helpers"

# Multi-step assertions from HuddleGrantTest. This deliberately keeps every
# intermediate row state: independent one-operation vectors miss stale objects,
# role transitions, restoration and a session moving between rooms.
class HuddleGrantSequenceOracle
  include ActiveSupport::Testing::TimeHelpers

  def issue(session = 7001, member = 9011)
    { op: "issue", session_id: session, membership_id: member }
  end

  def cases
    {
      "reuse" => [issue, {op: "travel", seconds: 1}, issue],
      "restore_membership" => [issue, {op: "destroy_member", id: 9011}, {op: "restore_member", id: 9011}, issue(7001, 9041)],
      "ineligible" => [issue(7001, 9012), {op: "delete_member", id: 9011}, issue],
      "stamp" => [issue, {op: "travel", seconds: 1800}, issue],
      "switch_room" => [issue, {op: "seen", id: 1, age: 0}, issue(7001, 9021), issue(7001, 9031)],
      "same_room" => [issue, {op: "seen", id: 1, age: 0}, issue],
      "participants" => [issue(7003, 9012), {op: "seen", id: 1, age: 0}, issue, {op: "seen", id: 2, age: 0}, issue(7002), {op: "seen", id: 3, age: 0}, issue(7004, 9013), {op: "participants"}, {op: "revoke", id: 2}, {op: "revoke", id: 3}, {op: "participants"}, {op: "seen", id: 1, age: 21}, {op: "participants"}],
      "stage_roles" => [issue, issue(7003, 9012)],
      "nonstage_role" => [issue],
      "demote" => [{op: "role", id: 9012, role: "speaker"}, issue(7003, 9012), issue, {op: "role", id: 9012, role: "listener"}],
      "host_speaker" => [{op: "role", id: 9012, role: "speaker"}, issue(7003, 9012), {op: "travel", seconds: 1}, {op: "role", id: 9012, role: "host"}, {op: "travel", seconds: 1}, {op: "role", id: 9012, role: "speaker"}],
      "authorize_mismatch" => [{op: "role", id: 9012, role: "speaker"}, issue(7003, 9012), {op: "authorize", id: 1}, {op: "raw_role", id: 9012, role: "listener"}, {op: "authorize", id: 1}],
      "rejoin_role" => [issue(7003, 9012), {op: "role", id: 9012, role: "speaker"}, issue(7003, 9012)]
    }
  end

  def run
    ActiveRecord::Schema.verbose = false
    ENV["LIVEKIT_API_KEY"] = "ws13b-fixture-api-key"
    ENV["LIVEKIT_API_SECRET"] = "ws13b-fixture-api-secret"
    ENV.delete("LIVEKIT_INTERNAL_URL")
    ENV.delete("LIVEKIT_URL")
    ActionCable.server.define_singleton_method(:broadcast) { |*_| }
    result = cases.map { |name, operations| scenario(name, operations) }
    puts JSON.pretty_generate(reference_pin: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], now: Time.utc(2026, 1, 1, 12).to_i, cases: result)
  ensure
    travel_back
  end

  def scenario(name, operations)
    travel_to Time.utc(2026, 1, 1, 12)
    load Rails.root.join("db/schema.rb")
    ActiveRecord::FixtureSet.reset_cache
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"), %w[accounts users rooms memberships])
    stage = %w[stage_roles demote host_speaker authorize_mismatch rejoin_role].include?(name)
    users = %w[david jason kevin].map { |key| User.find(ActiveRecord::FixtureSet.identify(key)) }
    rooms = [9001, 9002, 9003].map { |id| Room.create!(id: id, type: stage ? "Rooms::Stage" : "Rooms::Voice", name: "WS13b #{id}", creator: users[0]) }
    members = rooms.each_with_index.flat_map do |room, index|
      users.each_with_index.map { |user, offset| room.memberships.create!(id: 9011 + index * 10 + offset, user: user, involvement: "everything", stage_role: stage ? (offset == 0 ? "host" : "listener") : nil) }
    end
    sessions = [users[0], users[0], users[1], users[2]].each_with_index.map { |user, index| Session.create!(id: 7001 + index, user: user, token: "ws13b-sequence-session-#{index}") }
    input = { rooms: rooms.map(&:attributes), memberships: members.map(&:attributes), sessions: sessions.map(&:attributes) }
    results = operations.map do |op|
      value, error = nil, nil
      begin
        value = case op[:op]
        when "issue"
          # Keep the stale membership coordinates, like the Ruby declaration.
          member = members.find { |m| m.id == op[:membership_id] } || Membership.find(op[:membership_id])
          op[:room_id] = member.room_id
          HuddleGrant.issue!(session: Session.find(op[:session_id]), membership: member).id
        when "travel" then travel(op[:seconds].seconds); nil
        when "seen" then HuddleGrant.find(op[:id]).update_columns(last_seen_at: Time.current - op[:age]); nil
        when "destroy_member" then Membership.find(op[:id]).destroy!; nil
        when "delete_member" then Membership.find(op[:id]).delete; nil
        when "restore_member"
          old = members.find { |m| m.id == op[:id] }
          member = Membership.create!(id: 9041, user: old.user, room: old.room, involvement: "everything")
          op[:row] = member.attributes
          nil
        when "role" then Membership.find(op[:id]).change_stage_role!(op[:role]); nil
        when "raw_role" then Membership.find(op[:id]).update_columns(stage_role: op[:role]); nil
        when "authorize" then HuddleGrant.find(op[:id]).authorize_or_revoke!
        when "revoke" then HuddleGrant.find(op[:id]).revoke!; nil
        when "participants" then HuddleGrant.participants_for(rooms[0]).map(&:id)
        end
      rescue HuddleGrant::Ineligible => e
        error = e.class.name
      end
      grants = HuddleGrant.order(:id).map { |g| g.attributes.except("identity").merge("authorized" => g.authorized?, "in_call" => g.in_call?) }
      cleanups = HuddleCleanup.order(:id).map { |c| c.attributes.except("identity").merge("identity" => c.identity ? "grant:#{c.huddle_grant_id}" : nil) }
      { operation: op, value: value, error: error, grants: grants, cleanups: cleanups }
    end
    { name: name, input: input, results: results }
  end
end

HuddleGrantSequenceOracle.new.run
