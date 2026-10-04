require "json"
require "active_support/testing/time_helpers"

class HuddleDomainLifecycleSequences
  include ActiveSupport::Testing::TimeHelpers
  def issue(session = 7001, member = 9011) = {op: "issue", session_id: session, membership_id: member}
  def stream(member = 9011, room = 9001) = {op: "stream", membership_id: member, room_id: room}
  def role(id, value) = {op: "role", id: id, value: value}
  def remove(op = "destroy_member", id = 9011) = {op: op, id: id}
  def definitions
    speaker = [role(9012, "speaker"), stream(9012), issue(7003, 9012)]
    other_host = [role(9012, "host"), stream(9012), issue(7003, 9012)]
    [
      ["stage_scopes", {}, [{op: "scopes"}]],
      ["stage_later_member", {roster: %w[david]}, [{op: "add_member", user: "jason"}]],
      ["stage_reachable", {roster: %w[david]}, [{op: "chat"}, {op: "reachable"}]],
      ["deactivate_admin_successor", {administrator: "kevin"}, speaker + [remove("deactivate")]],
      ["deactivate_earliest_successor", {}, [remove("deactivate")]],
      ["deactivate_other_host", {}, other_host + [remove("deactivate")]],
      ["deactivate_empty", {roster: %w[david]}, [remove("deactivate")]],
      ["destroy_admin_successor", {administrator: "kevin"}, speaker + [issue(7004, 9013), {op: "chat"}, remove]],
      ["destroy_other_host", {}, other_host + [remove]],
      ["destroy_speaker", {}, speaker + [issue, remove("destroy_member", 9012)]],
      ["fresh_live_stream", {}, [stream, {op: "end", id: 40}, stream, {op: "live"}]],
      ["stream_live_scope", {}, [stream, stream(9021, 9002), {op: "end", id: 41}, {op: "live"}]],
      ["revoke_other_member", {}, [stream, issue(7003, 9012), remove("revoke", 1)]],
      ["authorization_ends_stream", {}, [stream, issue, {op: "grant_role", id: 1, value: "listener"}, {op: "authorize", id: 1}]],
      ["destroy_presenter_with_grant", {}, [stream, issue, role(9012, "host"), remove]],
      ["deactivate_presenter_with_grant", {}, [stream, issue, remove("deactivate")]],
      ["destroy_room_streams", {}, [stream, {op: "destroy_room"}]],
      ["voice_scopes", {type: "Rooms::Voice"}, [{op: "scopes"}]],
      ["voice_reachable", {type: "Rooms::Voice", roster: %w[david]}, [{op: "chat"}, {op: "reachable"}]],
      ["voice_deactivate", {type: "Rooms::Voice", roster: %w[david]}, [remove("deactivate")]]
    ]
  end
  def run
    ActiveRecord::Schema.verbose = false
    ENV["LIVEKIT_API_SECRET"] = "ws13b-fixture-api-secret"
    ENV["LIVEKIT_API_KEY"] = "ws13b-fixture-api-key"
    ENV.delete("LIVEKIT_URL"); ENV.delete("LIVEKIT_INTERNAL_URL")
    ActionCable.server.define_singleton_method(:broadcast) { |*_| }
    Huddle::PushInvitationJob.define_singleton_method(:perform_later) { |*_| }
    rows = definitions.map { |name, options, operations| scenario(name, options, operations) }
    puts JSON.pretty_generate(reference_pin: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], now: Time.utc(2026, 1, 1, 12).to_i, cases: rows)
  ensure
    travel_back
  end
  def scenario(name, options, operations)
    travel_to Time.utc(2026, 1, 1, 12)
    load Rails.root.join("db/schema.rb")
    # Schema reload leaves the virtual FTS table in place between scenarios.
    ActiveRecord::Base.connection.execute("DELETE FROM message_search_index")
    ActiveRecord::FixtureSet.reset_cache
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"), %w[accounts users rooms memberships])
    users = %w[david jason kevin].to_h { |key| [key, User.find(ActiveRecord::FixtureSet.identify(key))] }
    users["jason"].update_columns(role: :member)
    users["kevin"].update_columns(role: options[:administrator] == "kevin" ? :administrator : :member)
    type = options.fetch(:type, "Rooms::Stage")
    rooms = [9001, 9002].map { |id| Room.create!(id: id, type: type, name: "WS13b #{id}", creator: users["david"]) }
    roster = options.fetch(:roster, %w[david jason kevin])
    rooms.each_with_index do |room, index|
      roster.each_with_index { |key, offset| room.memberships.create!(id: 9011 + index * 10 + offset, user: users[key], involvement: "mentions", stage_role: type == "Rooms::Stage" ? (offset == 0 ? "host" : "listener") : nil, created_at: Time.current - (offset == 1 ? 2 : offset == 2 ? 1 : 0).days) }
    end
    # The side room must not keep David's stage memberships after deactivation:
    # it deliberately has the same roster, like the existing revocation slice.
    sessions = %w[david david jason kevin].each_with_index.map { |key, i| Session.create!(id: 7001 + i, user: users[key], token: "ws13b-lifecycle-session-#{i}") }
    input = {rooms: rooms.map(&:attributes), memberships: Membership.where(room_id: [9001, 9002]).map(&:attributes), users: users.values.map { |u| u.attributes.slice("id", "name", "role", "status", "inbox_preferences") }, sessions: sessions.map(&:attributes)}
    connection = ActiveRecord::Base.connection
    connection.execute("DELETE FROM main.sqlite_sequence WHERE name IN ('messages','streams')")
    connection.execute("INSERT INTO main.sqlite_sequence(name,seq) VALUES('messages',1200000000),('streams',39)")
    baseline = users["david"].memberships.without_direct_rooms.count - 2
    results = operations.map do |op|
      value = case op[:op]
      when "issue"
        member = Membership.find(op[:membership_id]); op[:room_id] = member.room_id
        HuddleGrant.issue!(session: Session.find(op[:session_id]), membership: member).id
      when "stream"
        member = Membership.find(op[:membership_id]); op[:user_id] = member.user_id
        Stream.create!(room: Room.find(op[:room_id]), membership: member, user: member.user, quality: "1080p15").id
      when "role" then Membership.find(op[:id]).change_stage_role!(op[:value]); nil
      when "grant_role" then HuddleGrant.find(op[:id]).update_columns(stage_role: op[:value]); nil
      when "authorize" then HuddleGrant.find(op[:id]).authorize_or_revoke!
      when "end" then Stream.find(op[:id]).end!; nil
      when "revoke" then HuddleGrant.find(op[:id]).revoke!; nil
      when "destroy_member" then Membership.find(op[:id]).destroy!; nil
      when "deactivate" then Membership.find(op[:id]).user.deactivate; nil
      when "destroy_room" then rooms[0].destroy!; nil
      when "add_member" then rooms[0].memberships.create!(user: users[op[:user]]).id
      when "chat" then rooms[0].messages.create!(creator: users["david"], body: "Hello from #{type == 'Rooms::Stage' ? 'stage' : 'voice'}", client_message_id: "ws13b-lifecycle-chat").id
      when "reachable" then %w[david jason].to_h { |key| [key, users[key].reachable_messages.exists?(id: 1200000001)] }
      when "scopes" then {channel: Room.without_directs.exists?(id: 9001), voice: Room.voices.exists?(id: 9001), stage: Room.where(type: "Rooms::Stage").exists?(id: 9001), membership_delta: users["david"].memberships.without_direct_rooms.count - baseline}
      when "live" then {ids: Stream.live.order(:id).pluck(:id), current: Room.find(9001).live_stream&.id}
      else raise op.inspect
      end
      {operation: op, value: value, grants: HuddleGrant.order(:id).map { |g| g.attributes.except("identity") }, streams: Stream.order(:id).map(&:attributes), memberships: Membership.where(room_id: [9001, 9002]).order(:id).map(&:attributes), rooms: Room.where(id: [9001, 9002]).order(:id).map { |r| r.attributes.slice("id", "deleted_at") }, messages: Message.where(room_id: [9001, 9002]).order(:id).map { |m| {id: m.id, room_id: m.room_id, creator_id: m.creator_id, system_note: m.system_note?, body: m.body.to_plain_text} }}
    end
    {name: name, input: input, results: results}
  end
end
HuddleDomainLifecycleSequences.new.run
