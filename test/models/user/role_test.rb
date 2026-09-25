require "test_helper"

class User::RoleTest < ActiveSupport::TestCase
  test "creating subsequent users makes them members" do
    assert User.create!(name: "User", email_address: "user@example.com", password: "secret123456").member?
  end

  test "can_administer?" do
    assert User.new(role: :administrator).can_administer?

    assert_not User.new(role: :member).can_administer?
    assert_not User.new.can_administer?
  end

  test "can administer a record" do
    member = User.new(role: :member)
    assert member.can_administer?(Room.new(creator: member))

    another_member = User.new(role: :member)
    assert another_member.can_administer?(Room.new(creator: member))
    assert_not another_member.can_administer?(rooms(:designers))
  end

  test "can delete a room" do
    admin = users(:david)
    creator = users(:kevin)
    member = users(:jz)

    assert admin.can_delete_room?(rooms(:designers))
    assert_not member.can_delete_room?(rooms(:designers))

    own = Rooms::Closed.create_for({ name: "Own", creator: creator }, users: [ creator, member ])
    assert creator.can_delete_room?(own)
    assert_not member.can_delete_room?(own)

    # Group DMs hold shared history: only administrators delete them,
    # even when a member created the group.
    group = Rooms::Direct.create_for({ name: "Group", creator: creator }, users: [ creator, member, admin ])
    assert admin.can_delete_room?(group)
    assert_not creator.can_delete_room?(group)
    assert_not member.can_delete_room?(group)
  end
end
