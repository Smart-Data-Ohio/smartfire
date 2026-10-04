require "json"
require "active_support/testing/time_helpers"
class HuddleJoinPushSequences
  include ActiveSupport::Testing::TimeHelpers
  def definitions
    [[1, ["push"]], [2, ["push", "push"]], [3, ["push", "later", "push"]],
     [4, ["dnd", "push"]], [5, ["dnd", "star", "push"]], [6, ["quiet", "push"]],
     [7, ["meeting", "push"]], [8, ["ooo", "push", "ooo_notify", "push"]],
     [9, ["connected", "push"]], [10, ["off", "push", "hidden", "push"]],
     [11, ["muted", "push"]], [12, ["inbox_off", "push"]], [13, ["no_subscriptions", "push"]]]
  end
  def run
    ActiveRecord::Schema.verbose = false
    ENV["LIVEKIT_API_SECRET"] = "ws13b-fixture-api-secret"
    ENV.delete("LIVEKIT_URL"); ENV.delete("LIVEKIT_INTERNAL_URL")
    ActionCable.server.define_singleton_method(:broadcast) { |*_| }
    Huddle::PushInvitationJob.define_singleton_method(:perform_later) { |*_| }
    titles = File.read(File.join(ENV.fetch("PARITY_WORK"), "reference-tools/huddle_pinned/huddle_join_pusher_test.rb")).scan(/^  test "(.*)" do$/).flatten
    cases = definitions.map do |number, ops|
      travel_to Time.utc(2026, 9, 23, 12)
      load Rails.root.join("db/schema.rb")
      ActiveRecord::FixtureSet.reset_cache
      ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"), %w[accounts users rooms memberships push/subscriptions])
      caller, recipient = %w[david jason].map { |key| User.find(ActiveRecord::FixtureSet.identify(key)) }
      room = Room.find(ActiveRecord::FixtureSet.identify(:david_and_jason))
      member = room.memberships.find_by!(user: recipient)
      grant = HuddleGrant.issue!(session: caller.sessions.create!(token: "ws13b-join-push-session"), membership: room.memberships.find_by!(user: caller))
      input = {recipient_id: recipient.id, sender_id: caller.id, room_id: room.id, membership_id: member.id, involvement: member.involvement, connected_at: member.connected_at, inbox_preferences: recipient.inbox_preferences.to_h, subscriptions: recipient.push_subscriptions.order(:id).pluck(:id)}
      results = ops.map do |op|
        pushes = []
        Rails.configuration.x.web_push_pool.define_singleton_method(:queue) { |payload, subscriptions| pushes << {payload: payload, subscription_ids: subscriptions.order(:id).pluck(:id)} }
        policy = nil
        case op
        when "push"
          policy = Notifications::Policy.new(recipient: recipient, sender: caller, kind: :huddle_join, room_membership: member).push?
          Huddle::JoinPusher.new(grant: grant, recipient: recipient, room_membership: member).push
        when "later" then travel 660.seconds
        when "dnd" then recipient.update!(dnd_enabled: true)
        when "star" then DndAllowedUser.create!(user: recipient, allowed_user: caller)
        when "quiet" then recipient.update!(quiet_hours_enabled: true, quiet_hours_start: "09:00", quiet_hours_end: "17:00")
        when "meeting"
          recipient.update!(meeting_status_enabled: true, meeting_dnd_enabled: true)
          Calendar::MeetingCache.create!(user: recipient, fetched_at: Time.current, busy_intervals: [[5.minutes.ago.iso8601,55.minutes.from_now.iso8601]])
        when "ooo" then recipient.update!(ooo_until: 1.day.from_now)
        when "ooo_notify" then recipient.update!(ooo_notify_enabled: true)
        when "connected" then member.connected
        when "off" then member.update!(involvement: "nothing")
        when "hidden" then member.update!(involvement: "invisible")
        when "muted" then member.update!(involvement: "muted")
        when "inbox_off" then recipient.update!(inbox_preferences: {"huddle_invitations" => false})
        when "no_subscriptions" then recipient.push_subscriptions.destroy_all
        end
        {operation: op, policy_allowed: policy, now: Time.current.to_i, involvement: member.reload.involvement, connected_at: member.connected_at, inbox_preferences: recipient.reload.inbox_preferences.to_h, subscription_ids: recipient.push_subscriptions.order(:id).pluck(:id), last_huddle_join_push_at: member.last_huddle_join_push_at, updated_at: member.updated_at, pushes: pushes}
      end
      {number: number, title: titles.fetch(number - 1), input: input, results: results}
    end
    puts JSON.pretty_generate(reference_pin: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], now: Time.utc(2026, 9, 23, 12).to_i, cases: cases)
  ensure
    travel_back
  end
end
HuddleJoinPushSequences.new.run
