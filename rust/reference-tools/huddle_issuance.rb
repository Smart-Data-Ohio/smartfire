require "json"
require "active_support/testing/time_helpers"

class HuddleIssuanceOracle
  include ActiveSupport::Testing::TimeHelpers

  def run
    ActiveRecord::Schema.verbose = false
    load Rails.root.join("db/schema.rb")
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"), %w[accounts users rooms memberships])
    ENV["LIVEKIT_API_KEY"] = "ws13-fixture-api-key"
    ENV["LIVEKIT_API_SECRET"] = "ws13-fixture-api-secret"
    ENV.delete("LIVEKIT_URL"); ENV.delete("LIVEKIT_INTERNAL_URL")
    @users = %w[david jason kevin bender].map { |key| User.find(ActiveRecord::FixtureSet.identify(key)) }
    variations = {
      "fresh_direct" => {}, "custom_group" => { name: "WS13 custom <&>" },
      "open" => { type: "Rooms::Open" }, "closed" => { type: "Rooms::Closed" },
      "voice" => { type: "Rooms::Voice" }, "stage" => { type: "Rooms::Stage" },
      "off" => { involvement: "nothing" }, "hidden" => { involvement: "invisible" },
      "muted" => { involvement: "muted" }, "inactive" => { viewer_status: :banned },
      "removed_member" => { removed: true }, "in_call_19" => { seen_age: 19 },
      "in_call_exact_20" => { seen_age: 20 }, "revoked_in_call" => { seen_age: 0, viewer_revoked: true },
      "suppressed" => { inbox: false }, "suppressed_quiet" => { inbox: false, quiet: true },
      "item_quiet" => { quiet: true }, "suppressed_custom_group" => { inbox: false, name: "WS13 custom <&>" },
      "owned_handled" => { reuse: true, item_age: 180, handled: true },
      "owned_read" => { reuse: true, item_age: 180, read: true },
      "owned_missed" => { reuse: true, item_age: 180, event_type: "huddle_missed" },
      "owned_other_type" => { reuse: true, item_age: 180, event_type: "mention" },
      "recent_item_exact_120" => { item_age: 120 }, "old_item_121" => { item_age: 121 },
      "recent_handled" => { item_age: 119, handled: true },
      "recent_missed" => { item_age: 119, event_type: "huddle_missed" },
      "recent_other_starter" => { item_age: 119, different_starter: true },
      "old_other_starter" => { item_age: 180, different_starter: true },
      "same_attempt_599" => { item_age: 599 }, "same_attempt_exact_600" => { item_age: 600 },
      "different_attempt_601" => { item_age: 601 }, "handled_attempt" => { item_age: 180, handled: true },
      "previous_issue_119" => { reuse: true, previous_issue_age: 119 },
      "previous_issue_exact_120" => { reuse: true, previous_issue_age: 120 },
      "previous_issue_121" => { reuse: true, previous_issue_age: 121 },
      "sibling_created_exact_120" => { sibling_age: 120 }, "sibling_created_121" => { sibling_age: 121 },
      "revoked_sibling_created_120" => { sibling_age: 120, sibling_revoked: true },
      "suppressed_reuse_recent" => { inbox: false, reuse: true, previous_issue_age: 120 },
      "suppressed_reuse_old" => { inbox: false, reuse: true, previous_issue_age: 121 },
      "owned_priority" => { reuse: true, item_age: 180, handled: true, other_item: true },
      "clear_started" => { incoming: "huddle_started" }, "clear_missed_read" => { incoming: "huddle_missed", read: true },
      "clear_only_this_room" => { incoming: "huddle_started", foreign_room: true },
      "no_humans" => { no_humans: true },
      "caller_banned" => { caller_status: :banned }, "caller_bot" => { caller_role: :bot },
      "deleted_room" => { deleted: true }, "wrong_session" => { wrong_session: true }
    }
    cases = variations.map { |name, options| scenario(name, options) }
    puts JSON.pretty_generate({ reference_pin: "d7c7de92", now: Time.utc(2026, 1, 1, 12).to_i, cases: cases })
  ensure
    Huddle::RingPolicy.quiet_check = nil
    travel_back
  end

  def scenario(name, options)
    travel_to Time.utc(2026, 1, 1, 12)
    ActionCable.server.define_singleton_method(:broadcast) { |*_| }
    @users.each(&:reload)
    room = Room.create!(id: 9001, type: options.fetch(:type, "Rooms::Direct"), name: options.fetch(:name, nil), creator: @users[0])
    # Open's committed create grants all active users; use the same controlled roster in each case.
    room.memberships.delete_all
    @users.each_with_index { |user, index| room.memberships.create!(id: 9011 + index, user: user, involvement: "everything", stage_role: options[:type] == "Rooms::Stage" ? "listener" : nil) }
    member = room.memberships.find_by!(user: @users[0])
    viewer = room.memberships.find_by!(user: @users[1])
    viewer.update_columns(involvement: options[:involvement]) if options[:involvement]
    @users[0].update_columns(status: options[:caller_status]) if options[:caller_status]
    @users[0].update_columns(role: options[:caller_role]) if options[:caller_role]
    @users[1].update_columns(status: options[:viewer_status]) if options[:viewer_status]
    @users[1].update_columns(inbox_preferences: { "huddle_invitations" => false }) if options[:inbox] == false
    room.memberships.where(user_id: [@users[1].id, @users[2].id]).delete_all if options[:no_humans]
    session = Session.create!(id: 7001, user: options[:wrong_session] ? @users[1] : @users[0], user_agent: "ws13-issuance")
    old_session = Session.create!(id: 7002, user: options[:different_starter] ? @users[2] : @users[0], user_agent: "ws13-old")
    if options[:reuse] || options.key?(:item_age) || options.key?(:sibling_age)
      starter = options[:different_starter] ? @users[2] : @users[0]
      source = HuddleGrant.create!(id: 17, identity: "ws13-old-participant", room_name: "ws13-old-room", user: starter, room: room, membership: room.memberships.find_by!(user: starter), session: options[:reuse] ? session : old_session, last_issued_at: Time.current - options.fetch(:previous_issue_age, 1000), created_at: Time.current - options.fetch(:sibling_age, 1000), revoked_at: options[:sibling_revoked] ? Time.current : nil, stage_role: member.stage_role)
      if options.key?(:item_age)
        item = ActivityItem.create!(id: 30, user: @users[1], source: source, event_type: options.fetch(:event_type, "huddle_started"), created_at: Time.current - options[:item_age])
        item.update_columns(read_at: Time.current - 80) if options[:read]
        item.update_columns(handled_at: Time.current - 75) if options[:handled]
      end
      if options[:other_item]
        other = HuddleGrant.create!(id: 18, identity: "ws13-other-device", room_name: "ws13-other-room", user: @users[0], room: room, membership: member, session: old_session, created_at: Time.current - 1000)
        ActivityItem.create!(id: 31, user: @users[1], source: other, event_type: "huddle_started", created_at: Time.current - 330)
      end
    end
    if options.key?(:seen_age) || options[:incoming]
      joined = HuddleGrant.create!(id: 50, identity: "ws13-viewer-grant", room_name: "ws13-viewer-room", user: @users[1], room: room, membership: viewer, session: Session.create!(id: 7003, user: @users[1]), last_seen_at: options.key?(:seen_age) ? Time.current - options[:seen_age] : nil, revoked_at: options[:viewer_revoked] ? Time.current : nil, created_at: Time.current - 1000)
      if options[:incoming]
        item = ActivityItem.create!(id: 32, user: @users[0], source: joined, event_type: options[:incoming], created_at: Time.current - 1000)
        item.update_columns(read_at: Time.current - 80) if options[:read]
        if options[:foreign_room]
          foreign = HuddleGrant.create!(id: 51, identity: "ws13-foreign-room", room_name: "ws13-foreign-room", user: @users[1], room: Room.find(ActiveRecord::FixtureSet.identify(:watercooler)), membership: Membership.find(ActiveRecord::FixtureSet.identify(:jason_watercooler)), session: joined.session)
          ActivityItem.create!(id: 33, user: @users[0], source: foreign, event_type: "huddle_started", created_at: Time.current - 1000)
        end
      end
    end
    room.memberships.where(user_id: @users[1].id).delete_all if options[:removed]
    room.update_columns(deleted_at: Time.current) if options[:deleted]
    input = {
      room: room.attributes,
      memberships: room.memberships.reload.map(&:attributes),
      users: @users.map { |user| user.reload.attributes.slice("id", "name", "status", "role", "inbox_preferences") },
      sessions: Session.where(id: [7001, 7002, 7003]).map(&:attributes),
      # Fixture loading creates a temporary sqlite_sequence that shadows the main table.
      grants: HuddleGrant.all.map(&:attributes), items: ActivityItem.all.map(&:attributes), sequences: HuddleGrant.uncached { HuddleGrant.connection.select_rows("SELECT name,seq FROM main.sqlite_sequence WHERE name IN ('huddle_grants','activity_items')").to_h }
    }
    broadcasts, jobs = [], []
    ActionCable.server.define_singleton_method(:broadcast) { |stream, payload| broadcasts << { stream: stream, payload: payload } }
    Huddle::PushInvitationJob.define_singleton_method(:perform_later) { |id| jobs << id }
    Huddle::RingPolicy.quiet_check = ->(_) { options[:quiet] || false }
    grant, error = nil, nil
    begin
      grant = HuddleGrant.issue!(session: session, membership: member)
    rescue HuddleGrant::Ineligible => exception
      error = exception.class.name
    end
    result = { name: name, input: input, session_id: session.id, membership_id: member.id, room_id: room.id, sound_allowed: !options[:quiet], grant_id: grant&.id, error: error, items: ActivityItem.order(:id).map(&:attributes), broadcasts: broadcasts, push_jobs: jobs }
    result[:grants] = HuddleGrant.order(:id).map { |row| row.attributes.except("identity") }
    result
  ensure
    ActivityItem.where(source_type: "HuddleGrant").delete_all
    HuddleCleanup.delete_all
    HuddleGrant.delete_all
    Membership.where(room_id: 9001).delete_all
    Room.where(id: 9001).delete_all
    Session.where(id: [7001, 7002, 7003]).delete_all
    @users[0].update_columns(status: :active, role: :administrator)
    @users[1].update_columns(status: :active, inbox_preferences: {})
  end
end
HuddleIssuanceOracle.new.run
