require "json"
require "active_support/testing/time_helpers"
class HuddleMembershipCreationOracle
  include ActiveSupport::Testing::TimeHelpers
  def run
    ActiveRecord::Schema.verbose = false
    travel_to Time.utc(2026, 1, 1, 12)
    cases = ["stage", "voice", "direct", "missing_room", "missing_user", "both_missing"].map do |name|
      load Rails.root.join("db/schema.rb")
      ActiveRecord::FixtureSet.reset_cache
      ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"), %w[accounts users rooms memberships])
      david, jason = %w[david jason].map { |key| User.find(ActiveRecord::FixtureSet.identify(key)) }
      type = {"stage" => "Rooms::Stage", "voice" => "Rooms::Voice"}.fetch(name, "Rooms::Direct")
      room = Room.create!(id: 9001, creator: david, type: type, name: "WS13b create")
      room.memberships.create!(id: 9011, user: david)
      room_id = %w[missing_room both_missing].include?(name) ? -1 : 9001
      user_id = %w[missing_user both_missing].include?(name) ? -1 : jason.id
      input = {room: room.reload.attributes, memberships: room.memberships.map(&:attributes), users: [david, jason].map { |u| u.attributes.slice("id", "name", "role", "status", "inbox_preferences") }, grants: [], items: []}
      member = Membership.new(room_id: room_id, user_id: user_id)
      error = nil
      begin
        member.save!
      rescue ActiveRecord::RecordInvalid => e
        error = e.record.errors.messages
      end
      {name: name, room_id: room_id, user_id: user_id, input: input, error: error, row: member.persisted? ? member.attributes : nil, direct_member_key: room.reload.direct_member_key}
    end
    puts JSON.pretty_generate(reference_pin: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], now: Time.current.to_i, cases: cases)
  ensure
    travel_back
  end
end
HuddleMembershipCreationOracle.new.run
