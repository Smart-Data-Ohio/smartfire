require "json"
require "active_support/testing/time_helpers"

# Replay JoinNotifier declarations as committed multi-step operations. Only external
# cable and job delivery are recorded; grant/item/resolver callbacks are real.
class HuddleNotifierSequences
  include ActiveSupport::Testing::TimeHelpers
  def issue(session = 7001, member = 9011)
    { op: "issue", session_id: session, membership_id: member, room_id: 9001 }
  end
  def seen(id = 1, age = 0) = { op: "seen", id: id, age: age }
  def at(seconds) = { op: "at", seconds: seconds }
  def item(op, user = "jason") = { op: op, user: user }
  def setting(op, value, user = "jason") = { op: op, value: value, user: user }
  def join(id = 1) = {op: "join", id: id}
  def missed(user = "jason") = {op: "resolve_ring", user: user}
  def definitions
    starter = [issue, seen]
    insiders = starter + [issue(7003, 9012), seen(2)]
    resolved = starter + [missed]
    [
      [1, {}, insiders + [join]], [2, {}, resolved + [join]], [3, {}, resolved + [join]],
      [4, {group: true}, insiders + [missed("kevin"), join]],
      [5, {roster: %w[david jason kevin jz]}, insiders + [missed("kevin"), missed("jz"), join]],
      [6, {type: "Rooms::Open", roster: %w[david jason bender]}, insiders + [join]],
      [7, {type: "Rooms::Open"}, starter + [join]],
      [8, {type: "Rooms::Voice"}, starter + [join, issue(7003, 9012), seen(2), join]],
      [9, {roster: %w[david jason bender]}, [setting("status", "deactivated"), issue, seen, join]],
      [10, {bot: true}, [join]],
      [11, {}, starter + [issue(7002), {op: "record_seen", id: 2}]],
      [12, {}, [issue, issue(7002), missed, {op: "record_seen", id: 1}, {op: "record_seen", id: 2}, {op: "perform_joins"}]],
      [13, {}, [issue, {op: "record_seen", id: 1}, missed, {op: "leave", id: 1}, {op: "perform_joins"}]],
      [14, {}, starter + [join]], [15, {}, starter + [at(61), seen, join]],
      [16, {}, resolved + [setting("involvement", "nothing"), join, setting("involvement", "invisible"), join]],
      [17, {}, resolved + [setting("inbox", false), join]],
      [18, {}, resolved + [setting("involvement", "muted"), join]],
      [19, {}, insiders + [{op: "revoke", id: 1}, issue, seen(3), join(3)]],
      [20, {}, insiders + [{op: "revoke", id: 1}, at(1), {op: "leave", id: 1}, at(0), issue, seen(3), join(3)]],
      [21, {}, [issue, issue(7003, 9012), seen(2), {op: "revoke", id: 1}, issue, seen(3), join(3)]],
      [22, {}, starter + [{op: "revoke", id: 1}, at(6), issue(7003, 9012), seen(2), issue, seen(3), join(3)]],
      [23, {}, starter + [{op: "revoke", id: 1}, at(10), issue(7003, 9012), seen(2), issue, seen(3), join(3)]],
      [24, {}, insiders + [{op: "leave", id: 1}]],
      [25, {}, insiders + [{op: "revoke", id: 1}]],
      [26, {group: true}, insiders + [{op: "leave", id: 1}]],
      [27, {group: true, type: "Rooms::Open"}, insiders + [{op: "leave", id: 1}]],
      [28, {}, starter + [{op: "leave", id: 1}]],
      [29, {type: "Rooms::Open"}, starter + [{op: "leave", id: 1}]],
      [30, {}, [issue, {op: "leave", id: 1}]], [31, {}, [issue, {op: "revoke", id: 1}]],
      [32, {}, insiders + [issue(7002), seen(3), {op: "leave", id: 3}]],
      [33, {}, insiders + [{op: "revoke", id: 1}, {op: "leave", id: 1}]]
    ]
  end
  def run
    ActiveRecord::Schema.verbose = false
    ENV["LIVEKIT_API_SECRET"] = "ws13b-fixture-api-secret"
    ENV["LIVEKIT_API_KEY"] = "ws13b-fixture-api-key"
    ENV.delete("LIVEKIT_URL"); ENV.delete("LIVEKIT_INTERNAL_URL")
    titles = File.read(Rails.root.join("test/models/huddle/join_notifier_test.rb")).scan(/^  test "(.*)" do$/).flatten
    rows = definitions.map { |number, options, operations| scenario(number, titles.fetch(number - 1), options, operations) }
    puts JSON.pretty_generate(reference_pin: "d7c7de92", now: Time.utc(2026, 1, 1, 12).to_i, cases: rows)
  ensure
    Huddle::RingPolicy.quiet_check = nil
    travel_back
  end
  def scenario(number, title, options, operations)
    @base = Time.utc(2026, 1, 1, 12)
    travel_to @base
    load Rails.root.join("db/schema.rb")
    ActiveRecord::FixtureSet.reset_cache
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"), %w[accounts users rooms memberships push/subscriptions])
    ActionCable.server.define_singleton_method(:broadcast) { |*_| }
    Huddle::PushInvitationJob.define_singleton_method(:perform_later) { |*_| }
    @users = %w[david jason kevin bender jz].to_h { |key| [key, User.find(ActiveRecord::FixtureSet.identify(key))] }
    roster = options[:roster] || (options[:group] ? %w[david jason kevin] : %w[david jason])
    room = Room.create!(id: 9001, type: options.fetch(:type, "Rooms::Direct"), name: options[:type] ? "Lounge" : nil, creator: @users["david"])
    room.memberships.delete_all
    roster.each_with_index { |key, i| room.memberships.create!(id: 9011 + i, user: @users[key], involvement: "everything") }
    sessions = %w[david david jason kevin jz david david].each_with_index.map { |key, i| Session.create!(id: 7001 + i, user: @users[key], token: "ws13b-invitation-session-#{i}") }
    if options[:bot]
      HuddleGrant.create!(id: 1, identity: "ws13b-bot-notice", room_name: "ws13b-bot-room", last_seen_at: Time.current, room: room, user: @users["bender"], session: sessions.first, membership: room.memberships.first)
    end
    pending = []
    input = {room: room.attributes, memberships: room.memberships.map(&:attributes), users: @users.values.map { |u| u.attributes.slice("id", "name", "role", "status", "inbox_preferences") }, sessions: sessions.map(&:attributes), grants: HuddleGrant.all.map(&:attributes), items: [], sequences: {}, subscriptions: Push::Subscription.all.map(&:attributes)}
    results = operations.map do |operation|
      broadcasts, jobs, pushes = [], [], []
      # Membership destruction also renders WS8's room removal and disconnects
      # cable. These declarations assert huddle JSON, which is our domain scope.
      ActionCable.server.define_singleton_method(:broadcast) { |stream, payload| broadcasts << {stream: stream, payload: payload} if stream.match?(/\Auser_\d+_(activity|huddle_notices)\z/) }
      Huddle::PushInvitationJob.define_singleton_method(:perform_later) { |id| jobs << {class: "Huddle::PushInvitationJob", id: id, delayed: false} }
      Huddle::BroadcastPresenceJob.define_singleton_method(:perform_later) { |id| jobs << {class: "Huddle::BroadcastPresenceJob", id: id, delayed: false} }
      Huddle::JoinNoticeJob.define_singleton_method(:perform_later) { |id| jobs << {class: "Huddle::JoinNoticeJob", id: id, delayed: false}; pending << id }
      Rails.configuration.x.web_push_pool.define_singleton_method(:queue) { |payload, subscriptions| pushes << {payload: payload, subscription_ids: subscriptions.order(:id).pluck(:id)} }
      Huddle::RingPolicy.quiet_check = ->(_) { options[:quiet] || false }
      value = case operation[:op]
      when "issue" then HuddleGrant.issue!(session: Session.find(operation[:session_id]), membership: Membership.find(operation[:membership_id])).id
      when "at" then travel_to(@base + operation[:seconds]); nil
      when "seen" then HuddleGrant.find(operation[:id]).update_columns(last_seen_at: Time.current - operation[:age]); nil
      when "join" then Huddle::JoinNotifier.notify_join(HuddleGrant.find(operation[:id])); nil
      when "resolve_ring" then ActivityItem.where(user: @users.fetch(operation[:user]), event_type: "huddle_started").update_all(event_type: "huddle_missed"); nil
      when "status" then @users.fetch(operation[:user]).update!(status: operation[:value]); nil
      when "record_seen" then HuddleGrant.find(operation[:id]).record_seen!; nil
      when "perform_joins" then pending.shift(pending.length).each { |id| Huddle::JoinNoticeJob.perform_now(id) }; nil
      when "inbox" then @users.fetch(operation[:user]).update!(inbox_preferences: {"huddle_invitations" => operation[:value]}); nil
      when "involvement" then room.memberships.find_by!(user: @users.fetch(operation[:user])).update!(involvement: operation[:value]); nil
      when "missed" then ActivityItem.find_by!(user: @users.fetch(operation[:user])).update!(event_type: "huddle_missed"); nil
      when "handled" then ActivityItem.find_by!(user: @users.fetch(operation[:user])).mark_handled!; nil
      when "resolve" then Huddle::InvitationResolver.resolve_overdue!; nil
      when "revoke" then HuddleGrant.find(operation[:id]).revoke!; nil
      when "leave" then HuddleGrant.find(operation[:id]).mark_out_of_call!
      when "destroy_member" then Membership.find(operation[:id]).destroy!; nil
      else raise operation.inspect
      end
      {operation: operation, value: value, items: ActivityItem.order(:id).map(&:attributes), grants: HuddleGrant.order(:id).map { |g| g.attributes.except("identity") }, broadcasts: broadcasts, jobs: jobs, pushes: pushes, throttle: room.memberships.order(:id).map { |m| {id: m.id, last_huddle_join_push_at: m.last_huddle_join_push_at} }}
    end
    {number: number, title: title, input: input, sound_allowed: !options[:quiet], results: results}
  end
end
HuddleNotifierSequences.new.run
