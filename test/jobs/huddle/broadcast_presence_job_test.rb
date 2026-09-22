require "test_helper"

class Huddle::BroadcastPresenceJobTest < ActiveSupport::TestCase
  setup do
    @environment_names = Huddle::REQUIRED_ENVIRONMENT
    @original_livekit_environment = ENV.values_at(*@environment_names)
    ENV["LIVEKIT_URL"] = "wss://huddle.example.test"
    ENV["LIVEKIT_INTERNAL_URL"] = "ws://livekit.example.test:7880"
    ENV["LIVEKIT_API_KEY"] = "test-api-key"
    ENV["LIVEKIT_API_SECRET"] = "test-api-secret"
    ENV["LIVEKIT_GATEWAY_SECRET"] = "test-gateway-secret"

    @room = Rooms::Voice.create_for({ name: "Lounge", creator: users(:david) }, users: [ users(:david), users(:jason) ])
    @grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: @room.memberships.find_by!(user: users(:david)))
  end

  teardown do
    @environment_names.zip(@original_livekit_environment).each { |name, value| ENV[name] = value }
  end

  test "broadcasts the room's current stacks" do
    @grant.update_columns(last_seen_at: Time.current)

    assert_difference -> { capture_turbo_stream_broadcasts([ users(:david), :rooms ]).count } do
      assert_difference -> { capture_turbo_stream_broadcasts([ users(:jason), :rooms ]).count } do
        assert_difference -> { capture_turbo_stream_broadcasts([ @room, :messages ]).count } do
          Huddle::BroadcastPresenceJob.perform_now(@grant.id)
        end
      end
    end
  end

  test "missing grants and rooms stay silent" do
    assert_no_difference -> { capture_turbo_stream_broadcasts([ @room, :messages ]).count } do
      Huddle::BroadcastPresenceJob.perform_now(-1)
    end

    @grant.update_columns(room_id: -1)
    assert_no_difference -> { capture_turbo_stream_broadcasts([ @room, :messages ]).count } do
      Huddle::BroadcastPresenceJob.perform_now(@grant.id)
    end
  end
end
