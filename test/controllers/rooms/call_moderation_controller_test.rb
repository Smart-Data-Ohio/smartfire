require "test_helper"

class Rooms::CallModerationControllerTest < ActionDispatch::IntegrationTest
  setup do
    @environment_names = Huddle::REQUIRED_ENVIRONMENT
    @original_livekit_environment = ENV.values_at(*@environment_names)
    ENV["LIVEKIT_URL"] = "wss://huddle.example.test"
    ENV["LIVEKIT_INTERNAL_URL"] = "ws://livekit.example.test:7880"
    ENV["LIVEKIT_API_KEY"] = "test-api-key"
    ENV["LIVEKIT_API_SECRET"] = "test-api-secret"
    ENV["LIVEKIT_GATEWAY_SECRET"] = "test-gateway-secret"

    @room = Rooms::Stage.create_for({ name: "Town Hall", creator: users(:jz) }, users: [ users(:jz), users(:kevin), users(:david) ])
    @host = @room.memberships.find_by!(user: users(:jz))
    @speaker = @room.memberships.find_by!(user: users(:kevin))
    @speaker.change_stage_role!("speaker")
  end

  teardown do
    @environment_names.zip(@original_livekit_environment).each do |name, value|
      ENV[name] = value
    end
  end

  test "a host server-mutes a speaker, revoking publish until unmuted" do
    session = users(:kevin).sessions.create!(user_agent: "Test")
    grant = HuddleGrant.issue!(session: session, membership: @speaker)
    sign_in :jz

    post room_call_moderation_mute_url(@room, @speaker)

    assert_redirected_to room_url(@room)
    assert_predicate @speaker.reload, :server_muted?
    assert grant.reload.revoked?
    assert HuddleCleanup.exists?(operation: :remove_participant, huddle_grant_id: grant.id)

    # The rejoin issues a subscribe-only token while the mute stands.
    muted = Huddle.new(room: @room, user: users(:kevin), session: session, membership: @speaker.reload)
    assert_predicate muted.grant, :server_muted?
    assert_not token_can_publish(muted.token)

    delete room_call_moderation_unmute_url(@room, @speaker)

    assert_redirected_to room_url(@room)
    assert_not_predicate @speaker.reload, :server_muted?
    assert muted.grant.reload.revoked?

    unmuted = Huddle.new(room: @room, user: users(:kevin), session: session, membership: @speaker.reload)
    assert_not_predicate unmuted.grant, :server_muted?
    assert token_can_publish(unmuted.token)
  end

  test "muting delivers a roster to every member and a rejoin event to the target" do
    sign_in :jz

    assert_difference -> { capture_turbo_stream_broadcasts([ users(:kevin), :rooms ]).count }, 2 do
      assert_difference -> { capture_turbo_stream_broadcasts([ users(:david), :rooms ]).count }, 1 do
        post room_call_moderation_mute_url(@room, @speaker)
      end
    end

    event = capture_turbo_stream_broadcasts([ users(:kevin), :rooms ]).find { |stream| stream["action"] == "append" }
    assert_equal "huddle_role_events", event["target"]

    roster_target = ActionView::RecordIdentifier.dom_id(@room, :stage_roster)
    roster = capture_turbo_stream_broadcasts([ users(:david), :rooms ]).find { |stream| stream["target"] == roster_target }
    assert_match "Muted", roster.to_html
    assert_match "Unmute", roster.to_html
  end

  test "muting twice and unmuting a member who was never muted both succeed" do
    sign_in :jz

    post room_call_moderation_mute_url(@room, @speaker)
    assert_redirected_to room_url(@room)
    post room_call_moderation_mute_url(@room, @speaker)
    assert_redirected_to room_url(@room)
    assert_predicate @speaker.reload, :server_muted?

    delete room_call_moderation_unmute_url(@room, @speaker)
    assert_redirected_to room_url(@room)
    delete room_call_moderation_unmute_url(@room, @speaker)
    assert_redirected_to room_url(@room)
    assert_not_predicate @speaker.reload, :server_muted?
  end

  test "a publish grant that survived a mute fails authorization" do
    session = users(:kevin).sessions.create!(user_agent: "Test")
    grant = HuddleGrant.issue!(session: session, membership: @speaker)

    # Bypass the mute callback the way a crashed transaction would: the
    # membership reads muted while the grant still reads publishable.
    @speaker.update_column(:server_muted_at, Time.current)

    assert_not grant.reload.authorized?
    assert_not grant.authorize_or_revoke!
    assert grant.reload.revoked?
    assert HuddleCleanup.exists?(operation: :remove_participant, huddle_grant_id: grant.id)
  end

  test "disconnect drops the member from the call but keeps the membership" do
    grant = HuddleGrant.issue!(session: users(:kevin).sessions.create!(user_agent: "Test"), membership: @speaker)
    grant.update_columns(last_seen_at: Time.current)
    sign_in :jz

    post room_call_moderation_disconnect_url(@room, @speaker)

    assert_redirected_to room_url(@room)
    assert grant.reload.revoked?
    assert @room.memberships.exists?(id: @speaker.id)

    # No rejoin event: the member stays out until they join again themselves.
    streams = capture_turbo_stream_broadcasts([ users(:kevin), :rooms ])
    assert_empty streams.select { |stream| stream["action"] == "append" && stream["target"] == "huddle_role_events" }
    assert_empty HuddleGrant.participants_for(@room)
  end

  test "an administrator who is not a host moderates a stage room" do
    sign_in :david

    post room_call_moderation_mute_url(@room, @speaker)
    assert_redirected_to room_url(@room)
    assert_predicate @speaker.reload, :server_muted?

    post room_call_moderation_disconnect_url(@room, @speaker)
    assert_redirected_to room_url(@room)
  end

  test "speakers and listeners cannot moderate" do
    sign_in :kevin
    post room_call_moderation_mute_url(@room, @host)
    assert_response :forbidden
    delete room_call_moderation_unmute_url(@room, @host)
    assert_response :forbidden
    post room_call_moderation_disconnect_url(@room, @host)
    assert_response :forbidden
    assert_not_predicate @host.reload, :server_muted?

    listener_room = Rooms::Stage.create_for({ name: "Small Stage", creator: users(:jz) }, users: [ users(:jz), users(:kevin) ])
    listener = listener_room.memberships.find_by!(user: users(:kevin))
    post room_call_moderation_mute_url(listener_room, listener_room.memberships.find_by!(user: users(:jz)))
    assert_response :forbidden
  end

  test "moderating your own session is rejected" do
    sign_in :jz

    post room_call_moderation_mute_url(@room, @host)
    assert_response :unprocessable_entity
    assert_not_predicate @host.reload, :server_muted?

    post room_call_moderation_disconnect_url(@room, @host)
    assert_response :unprocessable_entity
  end

  test "moderation is unreachable outside stage and voice rooms" do
    sign_in :david
    membership = memberships(:david_watercooler)

    post room_call_moderation_mute_url(membership.room, membership)
    assert_response :not_found
    delete room_call_moderation_unmute_url(membership.room, membership)
    assert_response :not_found
    post room_call_moderation_disconnect_url(membership.room, membership)
    assert_response :not_found
  end

  test "moderation denies outsiders, unknown memberships, and unauthenticated requests" do
    sign_in :kevin
    outsider_room = Rooms::Stage.create_for({ name: "Private Stage", creator: users(:david) }, users: [ users(:david) ])
    outsider_host = outsider_room.memberships.sole

    post room_call_moderation_mute_url(outsider_room, outsider_host)
    assert_response :not_found

    sign_in :jason
    post room_call_moderation_mute_url(outsider_room, outsider_host)
    assert_response :not_found

    sign_in :jz
    post room_call_moderation_mute_url(@room, 0)
    assert_response :not_found

    delete session_url
    post room_call_moderation_mute_url(@room, @speaker)
    assert_response :redirect
  end

  test "an administrator server-mutes a voice member, and members cannot" do
    voice = Rooms::Voice.create_for({ name: "Lounge", creator: users(:david) }, users: [ users(:david), users(:kevin) ])
    member = voice.memberships.find_by!(user: users(:kevin))
    session = users(:kevin).sessions.create!(user_agent: "Test")
    grant = HuddleGrant.issue!(session: session, membership: member)

    sign_in :kevin
    post room_call_moderation_mute_url(voice, voice.memberships.find_by!(user: users(:david)))
    assert_response :forbidden

    sign_in :david
    post room_call_moderation_mute_url(voice, member, format: :json)
    assert_response :no_content
    assert_predicate member.reload, :server_muted?
    assert grant.reload.revoked?

    muted = Huddle.new(room: voice, user: users(:kevin), session: session, membership: member.reload)
    assert_not token_can_publish(muted.token)

    event = capture_turbo_stream_broadcasts([ users(:kevin), :rooms ]).find { |stream| stream["action"] == "append" }
    assert_equal "huddle_role_events", event["target"]

    delete room_call_moderation_unmute_url(voice, member, format: :json)
    assert_response :no_content
    assert_not_predicate member.reload, :server_muted?
  end

  test "server mute only exists on stage and voice rooms" do
    membership = memberships(:david_watercooler)
    membership.server_muted_at = Time.current

    assert_not_predicate membership, :valid?
    assert_includes membership.errors[:server_muted_at], "only exists on stage and voice rooms"
  end

  private
    def token_can_publish(token)
      payload, = JWT.decode(token, nil, false)
      payload.dig("video", "canPublish") != false
    end
end
