require "json"
require "active_support/testing/time_helpers"

class HuddleGrantOracle
  include ActiveSupport::Testing::TimeHelpers

  def run
    ActiveRecord::Schema.verbose = false
    load Rails.root.join("db/schema.rb")
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"), %w[accounts users rooms memberships])
    ENV["LIVEKIT_API_KEY"] = "ws13-fixture-api-key"
    ENV["LIVEKIT_API_SECRET"] = "ws13-fixture-api-secret"
    ENV.delete("LIVEKIT_INTERNAL_URL")
    ENV.delete("LIVEKIT_GATEWAY_SECRET")
    travel_to Time.utc(2026, 1, 1, 12)
    david = User.find(ActiveRecord::FixtureSet.identify(:david))
    room = Room.find(ActiveRecord::FixtureSet.identify(:watercooler))
    membership = room.memberships.find_by!(user: david)
    session = david.sessions.create!(user_agent: "ws13")
    cases = []
    changes = {
      "active" => ->(g) {},
      "revoked" => ->(g) { g.update_columns(revoked_at: Time.current) },
      "missing_session" => ->(g) { Session.where(id: g.session_id).delete_all },
      "wrong_session_user" => ->(g) { Session.where(id: g.session_id).update_all(user_id: ActiveRecord::FixtureSet.identify(:jason)) },
      "removed_membership" => ->(g) { Membership.where(id: g.membership_id).delete_all },
      "wrong_membership_user" => ->(g) { Membership.where(id: g.membership_id).update_all(user_id: 999999) },
      "wrong_membership_room" => ->(g) { Membership.where(id: g.membership_id).update_all(room_id: 999999) },
      "banned_user" => ->(g) { User.where(id: g.user_id).update_all(status: :banned) },
      "deactivated_user" => ->(g) { User.where(id: g.user_id).update_all(status: :deactivated) },
      "bot_user" => ->(g) { User.where(id: g.user_id).update_all(role: :bot) },
      "missing_room" => ->(g) { Room.where(id: g.room_id).delete_all },
      "deleted_room" => ->(g) { Room.where(id: g.room_id).update_all(deleted_at: Time.current) },
      "server_mute_mismatch" => ->(g) { Membership.where(id: g.membership_id).update_all(server_muted_at: Time.current) },
      "server_mute_match" => ->(g) { Membership.where(id: g.membership_id).update_all(server_muted_at: Time.current); g.update_columns(server_muted: true) },
      "stage_role_mismatch" => ->(g) { Room.where(id: g.room_id).update_all(type: "Rooms::Stage"); Membership.where(id: g.membership_id).update_all(stage_role: :listener); g.update_columns(stage_role: "speaker"); g.reload },
      "stage_role_match" => ->(g) { Room.where(id: g.room_id).update_all(type: "Rooms::Stage"); Membership.where(id: g.membership_id).update_all(stage_role: :listener); g.update_columns(stage_role: "listener"); g.reload }
    }
    changes.each do |name, change|
      HuddleGrant.transaction(requires_new: true) do
        grant = HuddleGrant.create!(id: 17, identity: "ws13-security-participant", room_name: "ws13-security-room", session: session, user: david, membership: membership, room: room)
        change.call(grant)
        cases << { name: name, authorized: grant.reload.authorized? }
        raise ActiveRecord::Rollback
      end
    end
    liveness = []
    grant = HuddleGrant.create!(identity: "ws13-liveness-participant", room_name: "ws13-liveness-room", session: session, user: david, membership: membership, room: room)
    jobs = []
    Huddle::BroadcastPresenceJob.define_singleton_method(:perform_later) { |id| jobs << "presence" }
    Huddle::JoinNoticeJob.define_singleton_method(:perform_later) { |id| jobs << "join" }
    [0, 1, 9, 10, 29, 30, 49, 50].each do |seconds|
      travel_back; travel_to Time.utc(2026, 1, 1, 12) + seconds
      jobs.clear
      grant.record_seen!
      liveness << { at: Time.current.to_i, last_seen: grant.reload.last_seen_at.to_i, in_call: grant.in_call?, jobs: jobs.dup, updated_at: grant.updated_at.to_i }
    end
    puts JSON.pretty_generate({ reference_pin: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], now: Time.utc(2026, 1, 1, 12).to_i, authorization: cases, liveness: liveness })
  ensure
    travel_back
  end
end
HuddleGrantOracle.new.run
