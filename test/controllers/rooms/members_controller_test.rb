require "test_helper"

class Rooms::MembersControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
  end

  test "returns active room members with presence and status fields" do
    room = rooms(:designers)
    jason_session = users(:jason).sessions.create!(user_agent: "test", ip_address: "127.0.0.1")
    WorkspacePresenceLease.establish(user: users(:jason), session: jason_session)
    users(:jason).update!(custom_status_emoji: "🚂", custom_status_text: "On a train")

    get room_members_url(room, format: :json)

    assert_response :success
    members = response.parsed_body.fetch("members")
    assert_equal members.sort_by { |member| [ member.fetch("name").downcase, member.fetch("id") ] }, members
    assert_equal %w[ avatar_url bot id name online presence starred status ], members.first.keys.sort

    jason = members.find { |member| member["id"] == users(:jason).id }
    assert jason.fetch("online")
    assert_equal "online", jason.fetch("presence")
    assert_equal "🚂 On a train", jason.fetch("status")
    assert_not jason.fetch("bot")

    kevin = members.find { |member| member["id"] == users(:kevin).id }
    assert_not kevin.fetch("online")
    assert_equal "offline", kevin.fetch("presence")
    assert_nil kevin.fetch("status")
  end

  test "flags bots so the picker can exclude them from huddles" do
    get room_members_url(rooms(:watercooler), format: :json)

    assert_response :success
    members = response.parsed_body.fetch("members")
    assert members.find { |member| member["id"] == users(:bender).id }.fetch("bot")
    assert_not members.find { |member| member["id"] == users(:david).id }.fetch("bot")
  end

  test "reports idle and do-not-disturb presence" do
    jason_session = users(:jason).sessions.create!(user_agent: "test", ip_address: "127.0.0.1")
    lease = WorkspacePresenceLease.establish(user: users(:jason), session: jason_session)
    lease.update_column(:last_active_at, 11.minutes.ago)

    get room_members_url(rooms(:designers), format: :json)
    jason = response.parsed_body.fetch("members").find { |member| member["id"] == users(:jason).id }
    assert jason.fetch("online")
    assert_equal "idle", jason.fetch("presence")

    users(:jason).update!(presence_setting: "dnd")
    get room_members_url(rooms(:designers), format: :json)
    jason = response.parsed_body.fetch("members").find { |member| member["id"] == users(:jason).id }
    assert jason.fetch("online")
    assert_equal "dnd", jason.fetch("presence")

    users(:jason).update!(presence_setting: "invisible")
    get room_members_url(rooms(:designers), format: :json)
    jason = response.parsed_body.fetch("members").find { |member| member["id"] == users(:jason).id }
    assert_not jason.fetch("online")
    assert_equal "offline", jason.fetch("presence")
  end

  test "does not return inactive users" do
    users(:jason).deactivated!

    get room_members_url(rooms(:designers), format: :json)

    assert_response :success
    assert_not_includes response.parsed_body.fetch("members").pluck("id"), users(:jason).id
  end

  test "does not expose members of an inaccessible room" do
    private_room = Rooms::Closed.create!(name: "Private", creator: users(:jason))
    private_room.memberships.grant_to users(:jason)

    get room_members_url(private_room, format: :json)

    assert_response :not_found
    assert_not_includes response.body, users(:jason).name
  end

  test "starred is per viewer and never leaks between viewers" do
    users(:david).user_stars.create!(starred_user: users(:kevin))
    users(:jason).user_stars.create!(starred_user: users(:jz))

    get room_members_url(rooms(:designers), format: :json)
    assert_response :success
    david_view = response.parsed_body.fetch("members")
    assert david_view.find { |member| member["id"] == users(:kevin).id }.fetch("starred")
    assert_not david_view.find { |member| member["id"] == users(:jz).id }.fetch("starred")
    assert_not david_view.find { |member| member["id"] == users(:david).id }.fetch("starred")

    delete session_url
    sign_in :jason

    get room_members_url(rooms(:designers), format: :json)
    assert_response :success
    jason_view = response.parsed_body.fetch("members")
    assert jason_view.find { |member| member["id"] == users(:jz).id }.fetch("starred")
    assert_not jason_view.find { |member| member["id"] == users(:kevin).id }.fetch("starred")
  end

  test "requires authentication" do
    delete session_url

    get room_members_url(rooms(:designers), format: :json)

    assert_response :unauthorized
    assert_empty response.body
  end

  test "reports a bot with a checked-in agent as online" do
    rooms(:designers).memberships.grant_to users(:bender)
    agents(:bender_agent).update_column(:last_seen_at, Time.current)

    get room_members_url(rooms(:designers), format: :json)

    member = response.parsed_body.fetch("members").find { |item| item["id"] == users(:bender).id }
    assert member.fetch("online")
  end

  test "reports a bot as offline until its agent checks in" do
    rooms(:designers).memberships.grant_to users(:bender)

    get room_members_url(rooms(:designers), format: :json)

    member = response.parsed_body.fetch("members").find { |item| item["id"] == users(:bender).id }
    assert_not member.fetch("online")
  end

  test "reports a bot with a suspended agent as offline" do
    rooms(:designers).memberships.grant_to users(:bender)
    agents(:bender_agent).update_column(:last_seen_at, Time.current)
    agents(:bender_agent).suspend!

    get room_members_url(rooms(:designers), format: :json)

    member = response.parsed_body.fetch("members").find { |item| item["id"] == users(:bender).id }
    assert_not member.fetch("online")
  end

  test "revoked sessions immediately make a member offline" do
    jason_session = users(:jason).sessions.create!(user_agent: "test", ip_address: "127.0.0.1")
    WorkspacePresenceLease.establish(user: users(:jason), session: jason_session)
    Session.delete(jason_session.id)

    get room_members_url(rooms(:designers), format: :json)

    member = response.parsed_body.fetch("members").find { |item| item["id"] == users(:jason).id }
    assert_not member.fetch("online")
  end
end
