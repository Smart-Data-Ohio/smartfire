require "test_helper"

class Users::PresencesControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @jason_session = users(:jason).sessions.create!(user_agent: "Test", ip_address: "127.0.0.1")
  end

  test "returns presence and custom status for workspace users" do
    WorkspacePresenceLease.establish(user: users(:jason), session: @jason_session)
    users(:jason).update!(custom_status_emoji: "🚂", custom_status_text: "On a train")

    get presence_users_url(ids: [ users(:jason).id, users(:kevin).id, users(:bender).id, "nope" ])

    assert_response :success
    presences = response.parsed_body["presences"].index_by { |entry| entry["id"] }

    assert_equal "online", presences[users(:jason).id]["presence"]
    assert_equal "🚂 On a train", presences[users(:jason).id]["status"]
    assert_equal "offline", presences[users(:kevin).id]["presence"]
    assert_nil presences[users(:kevin).id]["status"]
    assert_not presences.key?(users(:bender).id)
  end

  test "an expired custom status reads as blank" do
    users(:jason).update!(custom_status_text: "Old", custom_status_expires_at: 1.hour.ago)

    get presence_users_url(ids: [ users(:jason).id ])

    assert_nil response.parsed_body["presences"].first["status"]
  end

  test "a presence lookup never prunes expired leases" do
    lease = WorkspacePresenceLease.establish(user: users(:jason), session: @jason_session)
    lease.update_column(:expires_at, 1.minute.ago)

    get presence_users_url(ids: [ users(:jason).id ])

    assert_response :success
    assert_equal "offline", response.parsed_body["presences"].first["presence"]
    assert WorkspacePresenceLease.exists?(lease.id),
      "pruning is the periodic sweep's job, not a hot read's"
  end

  test "invisible members read offline" do
    WorkspacePresenceLease.establish(user: users(:jason), session: @jason_session)
    users(:jason).update!(presence_setting: "invisible")

    get presence_users_url(ids: [ users(:jason).id ])

    assert_equal "offline", response.parsed_body["presences"].first["presence"]
  end
end
