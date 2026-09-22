require "test_helper"

class Internal::HuddleControllerTest < ActionDispatch::IntegrationTest
  setup do
    @environment_names = Huddle::REQUIRED_ENVIRONMENT
    @original_livekit_environment = ENV.values_at(*@environment_names)
    ENV["LIVEKIT_URL"] = "wss://huddle.example.test"
    ENV["LIVEKIT_INTERNAL_URL"] = "ws://livekit.example.test:7880"
    ENV["LIVEKIT_API_KEY"] = "test-api-key"
    ENV["LIVEKIT_API_SECRET"] = "test-api-secret"
    ENV["LIVEKIT_GATEWAY_SECRET"] = "test-gateway-secret"

    membership = memberships(:david_watercooler)
    @huddle = Huddle.new(room: membership.room, user: membership.user, session: sessions(:david_safari), membership: membership)
  end

  teardown do
    @environment_names.zip(@original_livekit_environment).each { |name, value| ENV[name] = value }
  end

  test "authorizes an exact active grant from an original token" do
    post_authorize(@huddle.token)

    assert_response :success
    assert_equal "no-store", response.headers["Cache-Control"]
    assert_equal expected_payload, response.parsed_body
  end

  test "authorizes a server-refreshed token that omits false permissions" do
    claims = decoded_claims(@huddle.token)
    claims.fetch("video").delete_if { |_permission, value| value == false }

    post_authorize(signed_token(claims))

    assert_response :success
    assert_equal expected_payload, response.parsed_body
  end

  test "rejects invalid signatures, missing time claims, and expired tokens" do
    claims = decoded_claims(@huddle.token)
    post_authorize(JWT.encode(claims, "wrong-secret", "HS256"))
    assert_response :unauthorized

    claims = decoded_claims(@huddle.token).except("nbf")
    post_authorize(signed_token(claims))
    assert_response :unauthorized

    claims = decoded_claims(@huddle.token).merge("exp" => 1.minute.ago.to_i)
    post_authorize(signed_token(claims))
    assert_response :unauthorized
  end

  test "rejects admin, data, metadata, and unknown privileges" do
    privileged_grants = [
      { "roomAdmin" => true },
      { "canPublishData" => true },
      { "canUpdateOwnMetadata" => true },
      { "unknownPrivilege" => true }
    ]

    privileged_grants.each do |privilege|
      claims = decoded_claims(@huddle.token)
      claims.fetch("video").merge!(privilege)
      post_authorize(signed_token(claims))
      assert_response :unauthorized
    end
  end

  test "rejects any publish source set other than the exact camera grant" do
    offending_source_sets = [
      %w[ microphone screen_share screen_share_audio ],
      %w[ camera ],
      [],
      Huddle::PUBLISH_SOURCES + [ "unknown_source" ],
      Huddle::PUBLISH_SOURCES + [ "camera" ]
    ]

    offending_source_sets.each do |sources|
      claims = decoded_claims(@huddle.token)
      claims.fetch("video")["canPublishSources"] = sources
      post_authorize(signed_token(claims))
      assert_response :unauthorized
    end
  end

  test "authorizes a stage listener token that cannot publish" do
    room = Rooms::Stage.create_for({ name: "Town Hall", creator: users(:david) }, users: [ users(:david), users(:jason) ])
    membership = room.memberships.find_by!(user: users(:jason))
    huddle = Huddle.new(room: room, user: users(:jason), session: users(:jason).sessions.create!(user_agent: "Test"), membership: membership)

    post_authorize(huddle.token)

    assert_response :success
    assert_equal(
      { "grant_id" => huddle.grant_id, "room_name" => huddle.room_name, "identity" => huddle.identity },
      response.parsed_body
    )
  end

  test "authorizes a listener token in server-refreshed shape" do
    room = Rooms::Stage.create_for({ name: "Town Hall", creator: users(:david) }, users: [ users(:david), users(:jason) ])
    membership = room.memberships.find_by!(user: users(:jason))
    huddle = Huddle.new(room: room, user: users(:jason), session: users(:jason).sessions.create!(user_agent: "Test"), membership: membership)

    claims = decoded_claims(huddle.token)
    claims.fetch("video").delete_if { |_permission, value| value == false }

    post_authorize(signed_token(claims))

    assert_response :success
  end

  test "rejects mismatched publish permission and sources" do
    mismatched_grants = [
      { "canPublish" => false, "canPublishSources" => Huddle::PUBLISH_SOURCES },
      { "canPublish" => false, "canPublishSources" => %w[ microphone ] },
      { "canPublish" => true, "canPublishSources" => [] },
      { "canPublish" => "yes", "canPublishSources" => [] }
    ]

    mismatched_grants.each do |grant|
      claims = decoded_claims(@huddle.token)
      claims.fetch("video").merge!(grant)
      post_authorize(signed_token(claims))
      assert_response :unauthorized
    end
  end

  test "rejects a publisher token missing its publish sources" do
    claims = decoded_claims(@huddle.token)
    claims.fetch("video").delete("canPublishSources")
    post_authorize(signed_token(claims))
    assert_response :unauthorized
  end

  test "authorizes a refreshed listener token that omits sources and the publish flag" do
    room = Rooms::Stage.create_for({ name: "Town Hall", creator: users(:david) }, users: [ users(:david), users(:jason) ])
    membership = room.memberships.find_by!(user: users(:jason))
    huddle = Huddle.new(room: room, user: users(:jason), session: users(:jason).sessions.create!(user_agent: "Test"), membership: membership)

    claims = decoded_claims(huddle.token)
    claims.fetch("video").delete("canPublishSources")
    post_authorize(signed_token(claims))
    assert_response :success

    claims = decoded_claims(huddle.token)
    claims.fetch("video").delete("canPublishSources")
    claims.fetch("video").delete("canPublish")
    post_authorize(signed_token(claims))
    assert_response :success
  end

  test "malformed signed video grants receive a controlled denial" do
    claims = decoded_claims(@huddle.token).merge("video" => "not-an-object")

    post_authorize(signed_token(claims))

    assert_response :unauthorized

    claims = decoded_claims(@huddle.token)
    claims.fetch("video")["canPublishSources"] = [ "microphone", 1 ]
    post_authorize(signed_token(claims))

    assert_response :unauthorized
  end

  test "a valid token without an exact database grant is forbidden" do
    claims = decoded_claims(@huddle.token).merge("sub" => "campfire-participant-missing")

    post_authorize(signed_token(claims))

    assert_response :forbidden
  end

  test "a stale database link revokes the grant and is forbidden" do
    Membership.where(id: @huddle.grant.membership_id).delete_all

    post_authorize(@huddle.token)

    assert_response :forbidden
    assert @huddle.grant.reload.revoked?
    assert HuddleCleanup.exists?(operation: :remove_participant, huddle_grant_id: @huddle.grant_id)
  end

  test "grant lookup rechecks current access without requiring or rechecking the original token" do
    travel 5.minutes do
      get "/internal/huddle/grants/#{@huddle.grant_id}", headers: gateway_headers
    end

    assert_response :success
    assert_equal expected_payload, response.parsed_body
    assert_equal "no-store", response.headers["Cache-Control"]
  end

  test "grant lookup returns not found after revocation" do
    @huddle.grant.revoke!

    get "/internal/huddle/grants/#{@huddle.grant_id}", headers: gateway_headers

    assert_response :not_found
  end

  test "a successful authorization records liveness without changing the response" do
    assert_nil @huddle.grant.last_seen_at

    freeze_time do
      post_authorize(@huddle.token)

      assert_response :success
      assert_equal expected_payload, response.parsed_body
      assert_equal Time.current, @huddle.grant.reload.last_seen_at
    end
  end

  test "grant lookup records liveness at most once per ten seconds" do
    get "/internal/huddle/grants/#{@huddle.grant_id}", headers: gateway_headers
    assert_response :success
    assert_equal expected_payload, response.parsed_body
    assert_not_nil @huddle.grant.reload.last_seen_at

    travel 9.seconds do
      assert_no_changes -> { @huddle.grant.reload.last_seen_at } do
        get "/internal/huddle/grants/#{@huddle.grant_id}", headers: gateway_headers
      end
      assert_response :success
    end

    travel 11.seconds do
      assert_changes -> { @huddle.grant.reload.last_seen_at } do
        get "/internal/huddle/grants/#{@huddle.grant_id}", headers: gateway_headers
      end
      assert_response :success
    end
  end

  test "a steady-state grant check runs no transaction and writes nothing" do
    # Seen recently, so the throttled liveness write is skipped too: this is
    # the per-second gateway check for a participant mid-call.
    @huddle.grant.update_columns(last_seen_at: Time.current)

    statements = []
    callback = ->(*, payload) { statements << payload[:sql] unless payload[:name] == "SCHEMA" }
    ActiveSupport::Notifications.subscribed(callback, "sql.active_record") do
      get "/internal/huddle/grants/#{@huddle.grant_id}", headers: gateway_headers
    end

    assert_response :success
    assert_empty statements.select { |sql| sql.match?(/\A\s*(begin|commit|rollback|savepoint)/i) },
      "expected no transaction statements, saw: #{statements.inspect}"
    assert_empty statements.select { |sql| sql.match?(/\A\s*(insert|update|delete)/i) },
      "expected no writes, saw: #{statements.inspect}"
    assert_operator statements.count, :<=, 8, "steady-state check ran: #{statements.inspect}"
  end

  test "a denied lookup takes the lock and revokes exactly once" do
    Membership.where(id: @huddle.grant.membership_id).delete_all

    statements = []
    callback = ->(*, payload) { statements << payload[:sql] unless payload[:name] == "SCHEMA" }
    ActiveSupport::Notifications.subscribed(callback, "sql.active_record") do
      get "/internal/huddle/grants/#{@huddle.grant_id}", headers: gateway_headers
    end

    assert_response :not_found
    assert @huddle.grant.reload.revoked?
    # Inside the test transaction the write lock arrives as a savepoint; in
    # production it is a top-level BEGIN IMMEDIATE. Either way the
    # double-checked lock revokes exactly once.
    assert_not_empty statements.select { |sql| sql.match?(/\A\s*(begin|savepoint)/i) },
      "expected the denied check to take the lock, saw: #{statements.inspect}"
    assert_equal 1, statements.count { |sql| sql.match?(/\A\s*update\s+"huddle_grants"/i) },
      "expected one revocation write, saw: #{statements.inspect}"
    assert_equal 1, statements.count { |sql| sql.match?(/\A\s*insert\s+into\s+"huddle_cleanups"/i) },
      "expected one cleanup row, saw: #{statements.inspect}"
  end

  test "a denied lookup does not record liveness" do
    @huddle.grant.revoke!

    get "/internal/huddle/grants/#{@huddle.grant_id}", headers: gateway_headers

    assert_response :not_found
    assert_nil @huddle.grant.reload.last_seen_at
  end

  test "a disconnect event marks the grant out of the call and refreshes presence" do
    @huddle.grant.update_columns(last_seen_at: 5.seconds.ago)

    assert_difference -> { capture_turbo_stream_broadcasts([ rooms(:watercooler), :messages ]).count } do
      post "/internal/huddle/grants/#{@huddle.grant_id}/left",
        params: { disconnected_at: Time.current.iso8601 }, headers: gateway_headers
    end

    assert_response :success
    assert_nil @huddle.grant.reload.last_seen_at
    assert_not @huddle.grant.revoked?
  end

  test "a stale disconnect event keeps a newer sighting" do
    @huddle.grant.update_columns(last_seen_at: Time.current)

    post "/internal/huddle/grants/#{@huddle.grant_id}/left",
      params: { disconnected_at: 1.minute.ago.iso8601 }, headers: gateway_headers

    assert_response :success
    assert_not_nil @huddle.grant.reload.last_seen_at
  end

  test "a disconnect event with no timestamp clears liveness" do
    @huddle.grant.update_columns(last_seen_at: Time.current)

    post "/internal/huddle/grants/#{@huddle.grant_id}/left", headers: gateway_headers

    assert_response :success
    assert_nil @huddle.grant.reload.last_seen_at
  end

  test "a disconnect event with a malformed timestamp is unprocessable" do
    @huddle.grant.update_columns(last_seen_at: Time.current)

    [ "2026-13-99", "not-a-timestamp" ].each do |disconnected_at|
      post "/internal/huddle/grants/#{@huddle.grant_id}/left",
        params: { disconnected_at: }, headers: gateway_headers

      assert_response :unprocessable_entity
      assert_not_nil @huddle.grant.reload.last_seen_at
    end
  end

  test "a disconnect event for an unknown grant is not found" do
    post "/internal/huddle/grants/-1/left", headers: gateway_headers

    assert_response :not_found
  end

  test "gateway authentication is required for every endpoint" do
    post "/internal/huddle/authorize", headers: { "Authorization" => "Bearer #{@huddle.token}" }
    assert_response :unauthorized

    get "/internal/huddle/grants/#{@huddle.grant_id}", headers: { "X-Huddle-Gateway-Secret" => "wrong" }
    assert_response :unauthorized

    post "/internal/huddle/grants/#{@huddle.grant_id}/left"
    assert_response :unauthorized

    post "/internal/huddle/grants/#{@huddle.grant_id}/left", headers: { "X-Huddle-Gateway-Secret" => "wrong" }
    assert_response :unauthorized
  end

  test "internal endpoints fail closed when huddles are not fully configured" do
    ENV.delete("LIVEKIT_INTERNAL_URL")

    post_authorize(@huddle.token)

    assert_response :service_unavailable
    assert_equal "no-store", response.headers["Cache-Control"]

    post "/internal/huddle/grants/#{@huddle.grant_id}/left", headers: gateway_headers
    assert_response :service_unavailable
  end

  private
    def post_authorize(token)
      post "/internal/huddle/authorize", headers: gateway_headers.merge("Authorization" => "Bearer #{token}")
    end

    def gateway_headers
      { "X-Huddle-Gateway-Secret" => "test-gateway-secret" }
    end

    def decoded_claims(token)
      JWT.decode(token, "test-api-secret", true, algorithm: "HS256").first
    end

    def signed_token(claims)
      JWT.encode(claims, "test-api-secret", "HS256")
    end

    def expected_payload
      {
        "grant_id" => @huddle.grant_id,
        "room_name" => @huddle.room_name,
        "identity" => @huddle.identity
      }
    end
end
