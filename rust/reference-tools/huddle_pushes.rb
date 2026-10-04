require "json"
require "active_support/testing/time_helpers"

class HuddlePushOracle
  include ActiveSupport::Testing::TimeHelpers
  def run
    ActiveRecord::Schema.verbose = false
    load Rails.root.join("db/schema.rb")
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"), %w[accounts users rooms memberships push/subscriptions])
    travel_to Time.utc(2026, 1, 1, 12)
    caller = User.find(ActiveRecord::FixtureSet.identify(:david))
    recipient = User.find(ActiveRecord::FixtureSet.identify(:jason))
    room = Room.find(ActiveRecord::FixtureSet.identify(:david_and_jason))
    member = room.memberships.find_by!(user: recipient)
    cases = []
    variants = {
      "default" => {}, "hidden" => { involvement: "invisible" }, "off" => { involvement: "nothing" },
      "muted" => { involvement: "muted" }, "mentions" => { involvement: "mentions" },
      "null_involvement" => { involvement: nil }, "no_subscriptions" => { no_subscriptions: true },
      "inbox_off" => { inbox: false }, "dnd" => { dnd: true },
      "fresh_connection" => { connected_age: 0 }, "exact_connection_ttl" => { connected_age: 60 },
      "stale_connection" => { connected_age: 61 }, "future_connection" => { connected_age: -1 },
      "disconnected_nonzero_count" => { connections: 7 }, "connected_zero_count" => { connections: 0, connected_age: 0 },
      "throttled_599" => { push_age: 599 }, "throttled_exact_600" => { push_age: 600 }, "throttled_601" => { push_age: 601 }
    }
    %w[huddle huddle_join].each do |kind|
      variants.each do |name, options|
        ActiveRecord::Base.transaction(requires_new: true) do
          member.update_columns(involvement: options.fetch(:involvement, "everything"), connections: options.fetch(:connections, 0), connected_at: options.key?(:connected_age) ? Time.current - options[:connected_age] : nil, last_huddle_join_push_at: options.key?(:push_age) ? Time.current - options[:push_age] : nil)
          recipient.update_columns(dnd_enabled: options.fetch(:dnd, false), inbox_preferences: options[:inbox] == false ? { "huddle_invitations" => false } : {})
          recipient.push_subscriptions.delete_all if options[:no_subscriptions]
          grant = HuddleGrant.create!(id: 17, identity: "ws13-push-participant", room_name: "ws13-push-room", session: caller.sessions.create!, user: caller, room: room, membership: room.memberships.find_by!(user: caller))
          item = ActivityItem.create!(id: 30, user: recipient, source: grant, event_type: "huddle_started")
          pushes = []
          Rails.configuration.x.web_push_pool.define_singleton_method(:queue) { |payload, subscriptions| pushes << { payload: payload, subscription_ids: subscriptions.order(:id).pluck(:id) } }
          policy = Notifications::Policy.new(recipient: recipient, sender: caller, kind: kind.to_sym, room_membership: kind == "huddle_join" ? member : nil).push?
          before = member.last_huddle_join_push_at&.to_i
          if kind == "huddle_join"
            Huddle::JoinPusher.new(grant: grant, recipient: recipient, room_membership: member).push
          else
            Huddle::InvitationPusher.new(activity_item: item).push
          end
          cases << { name: "#{kind}_#{name}", kind: kind, recipient_id: recipient.id, sender_id: caller.id, room_id: room.id, membership_id: member.id, options: options, policy_allowed: policy, before: before, after: member.reload.last_huddle_join_push_at&.to_i, pushes: pushes }
          raise ActiveRecord::Rollback
        end
        member.reload; recipient.reload
      end
    end
    puts JSON.pretty_generate({ reference_pin: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], now: Time.current.to_i, cases: cases })
  ensure
    travel_back
  end
end
HuddlePushOracle.new.run
