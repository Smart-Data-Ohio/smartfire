require "test_helper"

class Rooms::MembersControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
  end

  test "returns active room members and only minimal presence fields" do
    room = rooms(:designers)
    jason_session = users(:jason).sessions.create!(user_agent: "test", ip_address: "127.0.0.1")
    WorkspacePresenceLease.establish(user: users(:jason), session: jason_session)

    get room_members_url(room, format: :json)

    assert_response :success
    members = response.parsed_body.fetch("members")
    assert_equal members.sort_by { |member| [ member.fetch("name").downcase, member.fetch("id") ] }, members
    assert_equal %w[ avatar_url bot id name online ], members.first.keys.sort
    assert members.find { |member| member["id"] == users(:jason).id }.fetch("online")
    assert_not members.find { |member| member["id"] == users(:kevin).id }.fetch("online")
    assert_not members.find { |member| member["id"] == users(:jason).id }.fetch("bot")
  end

  test "flags bots so the picker can exclude them from huddles" do
    get room_members_url(rooms(:watercooler), format: :json)

    assert_response :success
    members = response.parsed_body.fetch("members")
    assert members.find { |member| member["id"] == users(:bender).id }.fetch("bot")
    assert_not members.find { |member| member["id"] == users(:david).id }.fetch("bot")
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
