require "json"
require "active_support/testing/time_helpers"

class HuddleResolverOracle
  include ActiveSupport::Testing::TimeHelpers
  def run
    ActiveRecord::Schema.verbose = false
    load Rails.root.join("db/schema.rb")
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"), %w[accounts users rooms memberships])
    ENV.delete("LIVEKIT_URL")
    @users = %w[david jason kevin bender].map { |key| User.find(ActiveRecord::FixtureSet.identify(key)) }
    Huddle::RingPolicy.quiet_check = ->(_) { false }
    variations = {
      "age_44" => { age: 44 }, "age_exact_45" => { age: 45 }, "age_46" => { age: 46 },
      "old_unread" => {}, "old_read" => { read: true }, "handled" => { handled: true },
      "already_missed" => { event: "huddle_missed" }, "other_event" => { event: "mention" },
      "missing_grant" => { missing: true }, "other_source" => { source: "Message" },
      "joined_exact_item" => { issued: 120 }, "joined_after_item" => { issued: 119 },
      "issued_before_item" => { issued: 121 }, "revoked_joined" => { issued: 120, revoked: true },
      "seen_19" => { seen: 19 }, "seen_exact_20" => { seen: 20 }, "revoked_seen" => { seen: 19, revoked: true },
      "other_room_join" => { issued: 0, foreign: true }, "other_user_join" => { issued: 0, other_user: true },
      "read_joined" => { read: true, issued: 0 }, "filter_recipient" => { filter: :jason },
      "filter_other_user" => { filter: :kevin }, "inactive_recipient" => { inactive: true },
      "voice" => { type: "Rooms::Voice" }, "stage" => { type: "Rooms::Stage" },
      "deleted_room" => { deleted: true }, "quiet_starter" => { starter_revoked: true },
      "group_unanswered" => { second: true }, "filter_two_recipients" => { second: true, filter: :jason }
    }
    cases = variations.map { |name, options| scenario(name, options) }
    puts JSON.pretty_generate({ reference_pin: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], now: Time.current.to_i, cases: cases })
  ensure
    Huddle::RingPolicy.quiet_check = nil
    travel_back
  end

  def scenario(name, options)
    travel_to Time.utc(2026,1,1,12)
    ActionCable.server.define_singleton_method(:broadcast) { |*_| }
    room = Room.create!(id: 9001, type: options.fetch(:type,"Rooms::Direct"), name: options[:type] ? "WS13 Lounge" : nil, creator: @users[0])
    @users.each_with_index { |u,i| room.memberships.create!(id: 9011+i, user: u, involvement: "everything") }
    source = HuddleGrant.create!(id: 17, identity: "ws13-resolver-starter",room_name: "ws13-room",user: @users[0],room: room,membership: room.memberships.find_by!(user: @users[0]),session: Session.create!(id: 7001,user: @users[0]), revoked_at: options[:starter_revoked] ? Time.current : nil)
    item = ActivityItem.create!(id: 30,user: @users[1],source: source,event_type: options.fetch(:event,"huddle_started"),created_at: Time.current - options.fetch(:age,120))
    item.update_columns(read_at: Time.current-80) if options[:read]
    item.update_columns(handled_at: Time.current-75) if options[:handled]
    item.update_columns(source_type: options[:source]) if options[:source]
    ActivityItem.create!(id:31,user:@users[2],source:source,event_type:"huddle_started",created_at:Time.current-120) if options[:second]
    if options.key?(:issued) || options.key?(:seen)
      user = options[:other_user] ? @users[2] : @users[1]
      joined_room = options[:foreign] ? Room.find(ActiveRecord::FixtureSet.identify(:watercooler)) : room
      HuddleGrant.create!(id: 50,identity: "ws13-resolver-recipient",room_name: "ws13-recipient-room",user: user,room: joined_room,membership: joined_room.memberships.find_by!(user: user),session: Session.create!(id: 7002,user: user),last_issued_at: options.key?(:issued) ? Time.current-options[:issued] : nil,last_seen_at: options.key?(:seen) ? Time.current-options[:seen] : nil,revoked_at: options[:revoked] ? Time.current : nil)
    end
    source.delete if options[:missing]
    @users[1].update_columns(status: :banned) if options[:inactive]
    room.update_columns(deleted_at: Time.current) if options[:deleted]
    input = {room: room.attributes,memberships: room.memberships.reload.map(&:attributes),users: @users.map { |u|u.reload.attributes.slice("id","name","role","status","inbox_preferences") },grants: HuddleGrant.all.map(&:attributes),items: ActivityItem.all.map(&:attributes)}
    broadcasts = []
    ActionCable.server.define_singleton_method(:broadcast) { |stream,payload|broadcasts << {stream: stream,payload: payload} }
    filter = options[:filter] ? User.find(ActiveRecord::FixtureSet.identify(options[:filter])) : nil
    Huddle::InvitationResolver.resolve_overdue!(user: filter)
    result = {name: name,input: input,user_id: filter&.id,items: ActivityItem.order(:id).map(&:attributes),broadcasts: broadcasts.dup}
    Huddle::InvitationResolver.resolve_overdue!(user: filter)
    result[:idempotent] = broadcasts.length == result[:broadcasts].length
    result
  ensure
    ActivityItem.where(id: [30,31]).delete_all
    HuddleGrant.delete_all
    Membership.where(room_id: 9001).delete_all
    Room.where(id: 9001).delete_all
    Session.where(id: [7001,7002]).delete_all
    @users[1].update_columns(status: :active)
  end
end
HuddleResolverOracle.new.run
