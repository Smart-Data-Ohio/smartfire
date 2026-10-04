require "json"
require "active_support/testing/time_helpers"

class HuddleNoticeOracle
  include ActiveSupport::Testing::TimeHelpers

  def run
    ActiveRecord::Schema.verbose = false
    load Rails.root.join("db/schema.rb")
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"), %w[accounts users rooms memberships push/subscriptions])
    travel_to Time.utc(2026, 1, 1, 12)
    ENV["LIVEKIT_API_SECRET"] = "ws13-fixture-api-secret"
    ENV.delete("LIVEKIT_URL") # Notice JSON is independent of presence rendering.
    @david, @jason, @kevin, @bot = %w[david jason kevin bender].map { |key| User.find(ActiveRecord::FixtureSet.identify(key)) }
    cases = []
    variations = {
      "direct_outsiders" => {}, "direct_in_call" => { viewer_seen: 0 },
      "direct_muted_outsider" => { involvement: "muted" },
      "direct_off_outsider" => { involvement: "nothing" },
      "direct_hidden_outsider" => { involvement: "invisible" },
      "direct_off_in_call" => { involvement: "nothing", viewer_seen: 0 },
      "direct_hidden_in_call" => { involvement: "invisible", viewer_seen: 0 },
      "direct_recent_ring" => { ring_age: 60 }, "direct_old_ring" => { ring_age: 61 },
      "direct_handled_ring" => { ring_age: 0, handled: true },
      "direct_missed_ring" => { ring_age: 0, event_type: "huddle_missed" },
      "direct_revoked_joiner" => { revoked: true },
      "direct_quiet_joiner" => { seen: 20 }, "direct_recent_joiner" => { seen: 19 },
      "direct_inactive_joiner" => { inactive: true }, "direct_inactive_viewer" => { viewer_inactive: true },
      "direct_rejoin" => { previous_revoke: 5, previous_seen: 20 },
      "direct_old_revoke" => { previous_revoke: 6, previous_seen: 20 },
      "direct_quiet_revoke" => { previous_revoke: 0, previous_seen: 21 },
      "direct_other_device" => { same_user_seen: 0 },
      "direct_custom_name" => { name: "WS13 group <&>" },
      "direct_non_ascii_name_space" => { joiner_name: "David\u00a0<&> Heinemeier" },
      "voice_outsiders" => { type: "Rooms::Voice" }, "voice_in_call" => { type: "Rooms::Voice", viewer_seen: 0 },
      "stage_outsiders" => { type: "Rooms::Stage" }, "stage_in_call" => { type: "Rooms::Stage", viewer_seen: 0 },
      "channel_outsiders" => { type: "Rooms::Open" }, "channel_in_call" => { type: "Rooms::Open", viewer_seen: 0 },
      "deleted_room" => { deleted: true }
    }
    [false, 0, 0.0, "0", "false", nil, true, "unexpected"].each_with_index do |value, index|
      variations["direct_pref_#{index}"] = { preferences_raw: value }
    end
    variations.each { |name, options| cases << scenario(name, "join", options) }
    {
      "direct_last_left" => {}, "direct_others_remain" => { viewer_seen: 0 },
      "direct_other_device_left" => { same_user_seen: 0 },
      "direct_inactive_leaver" => { inactive: true },
      "direct_hidden_outsider_left" => { viewer_seen: 0, involvement: "invisible" },
      "voice_last_left" => { type: "Rooms::Voice" }, "voice_others_remain" => { type: "Rooms::Voice", viewer_seen: 0 },
      "deleted_room_left" => { deleted: true }
    }.each { |name, options| cases << scenario(name, "leave", options) }
    {
      "ended_open_ring" => { ring_age: 0 }, "ended_read_ring" => { ring_age: 0, read: true },
      "ended_handled_ring" => { ring_age: 0, handled: true },
      "ended_others_remain" => { ring_age: 0, viewer_seen: 0 },
      "ended_suppressed" => { invitations: false }, "ended_suppressed_exact_window" => { invitations: false, issued_age: 60 },
      "ended_suppressed_recent" => { invitations: false, issued_age: 59 },
      "ended_inactive_recipient" => { ring_age: 0, viewer_inactive: true },
      "ended_missing_user" => { missing_user: true }, "ended_missing_room" => { missing_room: true },
      "ended_no_members" => { ring_age: 0, no_members: true }, "ended_deleted_room" => { ring_age: 0, deleted: true },
      "ended_non_ascii_name_space" => { ring_age: 0, joiner_name: "David\u00a0<&> Heinemeier" },
      "ended_another_starter" => { ring_age: 0, other_starter: true },
      "ended_own_item" => { ring_age: 0, own_item: true }, "ended_removed_invitee" => { ring_age: 0, removed_invitee: true }
    }.each { |name, options| cases << scenario(name, "ended", options) }
    {
      "revocation_live" => { viewer_seen: 0 }, "revocation_quiet" => { seen: 20 },
      "revocation_last" => { ring_age: 0 }
    }.each { |name, options| cases << scenario(name, "revoke", options.merge(callback: true)) }
    {
      "disconnect_live" => { viewer_seen: 0 }, "disconnect_revoked" => { revoked: true, viewer_seen: 0 },
      "disconnect_stale_floor" => { seen: 1, floor_age: 2 }
    }.each { |name, options| cases << scenario(name, "disconnect", options.merge(callback: true)) }
    puts JSON.pretty_generate({ reference_pin: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], now: Time.current.to_i, cases: cases })
  ensure
    travel_back
  end

  def scenario(name, operation, options)
    result = nil
    action = nil
    ActiveRecord::Base.transaction(requires_new: true) do
      room = Room.create!(id: 9001, type: options.fetch(:type, "Rooms::Direct"), name: options.fetch(:name, options.fetch(:type, "Rooms::Direct") == "Rooms::Direct" ? nil : "WS13 Lounge"), creator: @david)
      [@david, @jason, @kevin, @bot].each { |user| room.memberships.create!(user: user, involvement: "everything") }
      viewer = room.memberships.find_by!(user: @jason)
      viewer.update_columns(involvement: options[:involvement]) if options[:involvement]
      @david.update_columns(status: :deactivated) if options[:inactive]
      @david.update_columns(name: options[:joiner_name]) if options[:joiner_name]
      @jason.update_columns(status: :banned) if options[:viewer_inactive]
      @jason.update_columns(inbox_preferences: { "huddle_invitations" => options[:preferences_raw] }) if options.key?(:preferences_raw)
      @jason.update!(inbox_preferences: @jason.inbox_preferences.to_h.merge("huddle_invitations" => false)) if options[:invitations] == false
      seen = %w[join revoke disconnect].include?(operation) ? Time.current - options.fetch(:seen, 0) : nil
      grant = make_grant(17, room, @david, seen, revoked: options[:revoked], issued_age: options.fetch(:issued_age, 0))
      make_grant(18, room, @jason, Time.current - options[:viewer_seen]) if options.key?(:viewer_seen)
      make_grant(19, room, @david, Time.current - options[:same_user_seen]) if options.key?(:same_user_seen)
      if options.key?(:previous_revoke)
        old = make_grant(20, room, @david, Time.current - options[:previous_seen])
        old.update_columns(revoked_at: Time.current - options[:previous_revoke])
      end
      if options.key?(:ring_age)
        source = options[:other_starter] ? make_grant(21, room, @kevin, nil) : grant
        item = ActivityItem.create!(id: 30, user: @jason, source: source, event_type: options.fetch(:event_type, "huddle_started"), created_at: Time.current - options[:ring_age])
        item.update_columns(handled_at: Time.current) if options[:handled]
        item.update_columns(read_at: Time.current) if options[:read]
      end
      ActivityItem.create!(id: 31, user: @david, source: grant, event_type: "huddle_started") if options[:own_item]
      room.memberships.delete_all if options[:no_members]
      room.memberships.where(user_id: @jason.id).delete_all if options[:removed_invitee]
      room.update_columns(deleted_at: Time.current) if options[:deleted]
      grant.update_columns(user_id: -1) if options[:missing_user]
      grant.update_columns(room_id: -1) if options[:missing_room]
      input = { room: room.attributes.slice("id", "type", "name", "deleted_at"), grants: HuddleGrant.where(room_id: [room.id, -1]).map { |g| g.attributes }, memberships: room.memberships.map { |m| m.attributes.slice("id", "user_id", "room_id", "involvement") }, items: ActivityItem.where(source_type: "HuddleGrant").map(&:attributes), users: [@david, @jason, @kevin, @bot].map { |u| u.reload.attributes.slice("id", "name", "status", "role", "inbox_preferences") } }
      broadcasts, pushes = [], []
      ActionCable.server.define_singleton_method(:broadcast) { |stream, payload| broadcasts << { stream: stream, payload: payload } }
      Rails.configuration.x.web_push_pool.define_singleton_method(:queue) { |payload, subscriptions| pushes << { payload: payload, user_ids: subscriptions.order(:id).pluck(:user_id) } }
      grant.reload
      action = -> do
        case operation
      when "join" then Huddle::JoinNotifier.notify_join(grant)
      when "leave" then Huddle::JoinNotifier.notify_leave(grant)
      when "ended" then grant.send(:broadcast_call_ended_to_invitee)
      when "revoke" then grant.revoke!(create_cleanup: false)
      when "disconnect" then grant.mark_out_of_call!(seen_after: options[:floor_age] ? Time.current - options[:floor_age] : nil)
      end
        end
      action.call unless options[:callback]
      result = { name: name, operation: operation, grant_id: grant.id, input: input, floor: options[:floor_age] ? (Time.current - options[:floor_age]).to_i : nil, broadcasts: broadcasts, pushes: pushes }
      raise ActiveRecord::Rollback unless options[:callback]
    end
    if options[:callback]
      result[:broadcasts].clear
      result[:pushes].clear
      action.call
    end
    result
  ensure
    @david.reload; @jason.reload
    if options[:callback]
      ActivityItem.where(source_type: "HuddleGrant").delete_all
      HuddleCleanup.delete_all
      HuddleGrant.where(room_id: [9001, -1]).delete_all
      Membership.where(room_id: 9001).delete_all
      Room.where(id: 9001).delete_all
    end
  end

  def make_grant(id, room, user, seen, revoked: false, issued_age: 0)
    HuddleGrant.create!(id: id, identity: "ws13-notice-#{id}", room_name: "ws13-notice-room", session: user.sessions.create!(user_agent: "ws13-notice"), user: user, room: room, membership: room.memberships.find_by!(user: user), last_seen_at: seen, revoked_at: revoked ? Time.current : nil, last_issued_at: Time.current - issued_age)
  end
end
HuddleNoticeOracle.new.run
