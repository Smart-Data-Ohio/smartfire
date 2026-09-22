require "test_helper"

class Room::SoftDeleteAccessTest < ActiveSupport::TestCase
  setup do
    @original_api_secret = ENV["LIVEKIT_API_SECRET"]
    ENV["LIVEKIT_API_SECRET"] = "test-api-secret"
  end

  teardown do
    ENV["LIVEKIT_API_SECRET"] = @original_api_secret
  end

  test "a user created after the soft delete gains no membership in the deleted room" do
    room = rooms(:pets)
    room.begin_destroy!

    user = User.create!(name: "Newcomer", email_address: "newcomer@example.test", password: "secret123456")

    assert_not Membership.exists?(user: user, room: room)
    assert_empty user.rooms.where(id: room.id)
  end

  test "deleted rooms stay out of listings and search reach even with leftover memberships" do
    room = rooms(:pets)
    user = users(:david)
    message = room.root_messages.create!(creator: users(:jason), markdown_source: "Gone")
    room.begin_destroy!

    Membership.create!(room: room, user: user)

    assert_empty user.rooms.where(id: room.id)
    assert_empty user.reachable_messages.where(id: message.id)
  end

  test "huddle join and agent capabilities deny soft-deleted rooms" do
    room = rooms(:watercooler)
    room.begin_destroy!
    membership = Membership.create!(room: room, user: users(:david))

    assert_raises(HuddleGrant::Ineligible) do
      HuddleGrant.issue!(session: sessions(:david_safari), membership: membership)
    end

    legacy_grant = HuddleGrant.create!(identity: "legacy-grant", room_name: "legacy",
      session: sessions(:david_safari), user: users(:david), membership: membership, room: room)
    assert_not legacy_grant.authorized?

    agent = agents(:bender_agent)
    AgentGrant.create!(agent: agent, room: room, granted_by: users(:david), capability: "read_messages")
    assert_not agent.can?(:read_messages, room)

    # The same grant in a live room still allows.
    live_room = rooms(:designers)
    AgentGrant.create!(agent: agent, room: live_room, granted_by: users(:david), capability: "read_messages")
    assert agent.can?(:read_messages, live_room)
  end

  test "converting a deleted room to open grants no memberships" do
    room = rooms(:designers)
    room.begin_destroy!

    assert_no_difference -> { Membership.count } do
      room.becomes!(Rooms::Open).update!(name: "Reopened")
    end
  end

  test "direct lookup skips soft-deleted rooms" do
    room = rooms(:david_and_kevin)
    users = room.users.to_a
    room.begin_destroy!
    users.each { |user| Membership.create!(room: room, user: user) }

    found = Current.set(user: users(:david)) { Rooms::Direct.find_or_create_for(users) }

    assert_not_equal room.id, found.id
  end
end
