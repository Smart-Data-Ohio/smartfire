require "json"
require "active_support/testing/time_helpers"

class HuddlePresenceOracle
  include ActiveSupport::Testing::TimeHelpers

  def run
    ActiveRecord::Schema.verbose = false
    load Rails.root.join("db/schema.rb")
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"), %w[accounts users rooms memberships])
    ENV["LIVEKIT_URL"] = "wss://public.example.test"
    ENV["LIVEKIT_INTERNAL_URL"] = "http://internal.example.test:7880"
    ENV["LIVEKIT_API_KEY"] = "ws13-fixture-api-key"
    ENV["LIVEKIT_API_SECRET"] = "ws13-fixture-api-secret"
    ENV["LIVEKIT_GATEWAY_SECRET"] = "ws13-fixture-gateway-secret"
    Rails.application.routes.default_url_options[:host] = "example.org"
    travel_to Time.utc(2026, 1, 1, 12)
    rooms = %w[Rooms::Open Rooms::Closed Rooms::Direct Rooms::Voice Rooms::Stage].map do |type|
      Room.create!(type: type, name: "WS13 <room>", creator: User.first)
    end
    users = 6.times.map { |i| User.create!(name: ["A <&> \"one\"", "Beta", "Charlie", "Delta", "Echo", "Foxtrot"][i], email_address: "ws13-presence-#{i}@example.test", password: "ws13-fixture-password") }
    vectors = []
    rooms.each do |room|
      %w[sidebar header].each do |placement|
        [0, 1, 2, 3, 6].each do |count|
          participants = users.first(count)
          vectors << {
            type: room.type, room_id: room.id, placement: placement, participants: participants.map { |u| { id: u.id, name: u.name, avatar_path: Rails.application.routes.url_helpers.fresh_user_avatar_path(u) } },
            html: ApplicationController.render(partial: "rooms/huddles/participants", locals: { room: room, placement: placement.to_sym, participants: participants })
          }
        end
      end
    end
    puts JSON.pretty_generate({ reference_pin: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], cases: vectors })
  ensure
    travel_back
  end
end
HuddlePresenceOracle.new.run
