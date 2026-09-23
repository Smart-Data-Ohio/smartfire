require "test_helper"

class MembershipNavigationTest < ActiveSupport::TestCase
  setup do
    @room = rooms(:designers)
    @membership = memberships(:david_designers)
  end

  test "read clears unread and points at the newest root message" do
    first, second = @room.root_messages.ordered.first(2)
    @membership.update!(unread_at: second.created_at, last_read_message_id: first.id)

    @membership.read

    assert_not @membership.unread?
    assert_equal @room.root_messages.ordered.last.id, @membership.last_read_message_id
    assert_nil @membership.first_unread_message
    assert_equal 0, @membership.unread_count
  end

  test "first unread follows the pointer" do
    first, second, third = @room.root_messages.ordered.first(3)
    @membership.update!(unread_at: third.created_at, last_read_message_id: first.id)

    assert_equal second.id, @membership.first_unread_message.id
    assert_equal 2, @membership.unread_count
  end

  test "mark unread moves the pointer to just before the message" do
    @membership.read
    target = @room.root_messages.ordered.second

    @membership.mark_unread_before(target)

    assert_predicate @membership.reload, :unread?
    assert_equal target.id, @membership.first_unread_message.id
  end

  test "legacy unread rows without a pointer fall back to the unread stamp" do
    second = @room.root_messages.ordered.second
    @membership.update!(unread_at: second.created_at, last_read_message_id: nil)

    assert_equal second.id, @membership.first_unread_message.id
    assert_equal 2, @membership.unread_count
    assert_predicate @membership, :unread?
  end

  test "first unread is nil when read even with a stale pointer" do
    first = @room.root_messages.ordered.first
    @membership.update!(unread_at: nil, last_read_message_id: first.id)

    assert_nil @membership.first_unread_message
    assert_equal 0, @membership.unread_count
  end

  test "favourite appends positions and move compacts them" do
    hq = users(:david).memberships.find_by!(room: rooms(:hq))
    pets = users(:david).memberships.find_by!(room: rooms(:pets))

    @membership.favorite!
    hq.favorite!
    pets.favorite!

    assert_equal [ @membership.id, hq.id, pets.id ], users(:david).memberships.favorites.pluck(:id)

    pets.move_favorite_to(0)
    assert_equal [ pets.id, @membership.id, hq.id ], users(:david).memberships.favorites.pluck(:id)

    hq.unfavorite!
    assert_equal [ pets.id, @membership.id ], users(:david).memberships.favorites.pluck(:id)
  end

  test "move clamps and ignores non-favourites" do
    @membership.favorite!

    @membership.move_favorite_to(99)
    assert_equal 0, @membership.reload.favorite_position

    other = users(:david).memberships.find_by!(room: rooms(:hq))
    other.move_favorite_to(0)
    assert_not other.reload.favorited?
  end

  test "room category must belong to the member" do
    other = users(:jason).room_categories.create!(name: "Theirs", position: 1)
    @membership.room_category = other

    assert_not @membership.valid?
    assert_includes @membership.errors[:room_category], "must belong to the member"
  end
end
