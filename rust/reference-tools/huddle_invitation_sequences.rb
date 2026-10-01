require "json"
require "active_support/testing/time_helpers"

# Replay the declarations as committed multi-step operations. Only external
# cable and job delivery are recorded; grant/item/resolver callbacks are real.
class HuddleInvitationSequences
  include ActiveSupport::Testing::TimeHelpers
  def issue(session = 7001, member = 9011)
    { op: "issue", session_id: session, membership_id: member, room_id: 9001 }
  end
  def seen(id = 1, age = 0) = { op: "seen", id: id, age: age }
  def at(seconds) = { op: "at", seconds: seconds }
  def item(op, user = "jason") = { op: op, user: user }
  def setting(op, value, user = "jason") = { op: op, value: value, user: user }
  def definitions
    start = [issue, seen, issue(7003, 9012), seen(2)]
    [
      [1, {}, [issue]], [2, {}, [issue]], [3, { quiet: true }, [issue]],
      [4, {}, [setting("involvement", "nothing"), issue, setting("involvement", "invisible"), issue(7002)]],
      [5, {}, [setting("inbox", false), issue, at(46), {op: "resolve"}]],
      [6, {}, [setting("inbox", false), issue, issue(7002), issue(7006), at(180), issue(7007)]],
      [7, {}, [setting("inbox", false), issue, issue]],
      [8, { type: "Rooms::Open" }, [issue]], [9, { type: "Rooms::Voice" }, [issue]],
      [10, {}, [issue(7003, 9012), seen, issue]], [11, {}, [issue(7003, 9012), seen(1, 21), issue]],
      [12, {}, [issue, at(180), issue]], [13, {}, [issue, issue(7002)]],
      [14, {}, [issue, item("missed"), issue(7002)]], [15, {}, [issue, item("handled"), issue(7002)]],
      [16, {}, [at(-180), issue, at(0), issue]],
      [17, {}, [at(-180), issue, item("handled"), at(0), issue]],
      [18, {}, [at(-180), issue, item("missed"), at(0), issue]],
      [19, {}, [issue, seen, {op: "revoke", id: 1}]],
      [20, {}, [issue, seen, {op: "leave", id: 1}]], [21, {}, [issue, {op: "revoke", id: 1}]],
      [22, {}, [issue, issue(7003, 9012), seen, {op: "revoke", id: 1}]],
      [23, {}, [setting("inbox", false), issue, seen, {op: "revoke", id: 1}]],
      [24, {}, [setting("inbox", false), issue, at(300), seen, {op: "revoke", id: 1}]],
      [25, { group: true }, start + [{op: "leave", id: 1}]],
      [26, { group: true }, start + [{op: "revoke", id: 1}]],
      [27, { group: true }, start + [{op: "leave", id: 1}, {op: "leave", id: 2}]],
      [28, { group: true }, start + [{op: "leave", id: 1}, at(46), {op: "resolve"}]],
      [29, {}, [issue, at(46), {op: "resolve"}, at(180), issue(7002), at(360), {op: "resolve"}]],
      [30, {}, [issue, item("handled"), at(180), issue(7002)]],
      [31, {}, [issue, issue(7003, 9012), at(180), issue(7002), at(360), issue]],
      [32, {}, [at(-1200), issue, at(0), {op: "resolve"}, at(-660), issue(7002), at(0), issue(7006)]],
      [33, {}, [issue, issue(7003, 9012)]],
      [34, {}, [issue, at(46), {op: "resolve"}, at(0), issue(7003, 9012)]],
      [35, { roster: %w[david bender] }, [issue]],
      [36, { group: true }, [issue]],
      [37, { roster: %w[david jason kevin bender] }, [setting("involvement", "nothing", "kevin"), issue]],
      [38, { roster: %w[david jason kevin jz] }, [issue(7004, 9013), {op: "destroy_member", id: 9013}, issue]]
    ]
  end
  def run
    ActiveRecord::Schema.verbose = false
    ENV["LIVEKIT_API_SECRET"] = "ws13b-fixture-api-secret"
    ENV["LIVEKIT_API_KEY"] = "ws13b-fixture-api-key"
    ENV.delete("LIVEKIT_URL"); ENV.delete("LIVEKIT_INTERNAL_URL")
    titles = File.read(Rails.root.join("test/models/huddle_invitation_test.rb")).scan(/^  test "(.*)" do$/).flatten
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
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"), %w[accounts users rooms memberships])
    ActionCable.server.define_singleton_method(:broadcast) { |*_| }
    Huddle::PushInvitationJob.define_singleton_method(:perform_later) { |*_| }
    @users = %w[david jason kevin bender jz].to_h { |key| [key, User.find(ActiveRecord::FixtureSet.identify(key))] }
    roster = options[:roster] || (options[:group] ? %w[david jason kevin] : %w[david jason])
    room = Room.create!(id: 9001, type: options.fetch(:type, "Rooms::Direct"), name: options[:type] ? "Lounge" : nil, creator: @users["david"])
    room.memberships.delete_all
    roster.each_with_index { |key, i| room.memberships.create!(id: 9011 + i, user: @users[key], involvement: "everything") }
    sessions = %w[david david jason kevin jz david david].each_with_index.map { |key, i| Session.create!(id: 7001 + i, user: @users[key], token: "ws13b-invitation-session-#{i}") }
    input = {room: room.attributes, memberships: room.memberships.map(&:attributes), users: @users.values.map { |u| u.attributes.slice("id", "name", "role", "status", "inbox_preferences") }, sessions: sessions.map(&:attributes), grants: [], items: [], sequences: {}}
    results = operations.map do |operation|
      broadcasts, jobs = [], []
      # Membership destruction also renders WS8's room removal and disconnects
      # cable. These declarations assert huddle JSON, which is our domain scope.
      ActionCable.server.define_singleton_method(:broadcast) { |stream, payload| broadcasts << {stream: stream, payload: payload} if stream.match?(/\Auser_\d+_(activity|huddle_notices)\z/) }
      Huddle::PushInvitationJob.define_singleton_method(:perform_later) { |id| jobs << {class: "Huddle::PushInvitationJob", id: id, delayed: false} }
      Huddle::RingPolicy.quiet_check = ->(_) { options[:quiet] || false }
      value = case operation[:op]
      when "issue" then HuddleGrant.issue!(session: Session.find(operation[:session_id]), membership: Membership.find(operation[:membership_id])).id
      when "at" then travel_to(@base + operation[:seconds]); nil
      when "seen" then HuddleGrant.find(operation[:id]).update_columns(last_seen_at: Time.current - operation[:age]); nil
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
      {operation: operation, value: value, items: ActivityItem.order(:id).map(&:attributes), grants: HuddleGrant.order(:id).map { |g| g.attributes.except("identity") }, broadcasts: broadcasts, jobs: jobs}
    end
    {number: number, title: title, input: input, sound_allowed: !options[:quiet], results: results}
  end
end
HuddleInvitationSequences.new.run
