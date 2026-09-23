require "test_helper"

class Rooms::DirectTest < ActiveSupport::TestCase
  setup do
    Current.user = users(:david)
  end

  teardown do
    Current.user = nil
  end

  test "create room for same users" do
    room = Rooms::Direct.find_or_create_for([ users(:david), users(:kevin) ])
    assert room.users.include?(users(:david))
    assert room.users.include?(users(:kevin))
    assert_not room.users.include?(users(:jason))
  end

  test "only one room will exist for the same users" do
    room1 = Rooms::Direct.find_or_create_for([ users(:david), users(:kevin) ])
    room2 = Rooms::Direct.find_or_create_for([ users(:kevin), users(:david) ])
    assert_equal room1, room2
  end

  test "default involvement for new users" do
    room = Rooms::Direct.find_or_create_for([ users(:david), users(:kevin) ])
    assert room.memberships.all? { |m| m.involved_in_everything? }
  end

  test "created rooms carry the hash of their exact member set" do
    room = Rooms::Direct.find_or_create_for([ users(:jz), users(:david) ])

    assert_equal Rooms::Direct.member_key_for([ users(:david).id, users(:jz).id ]), room.direct_member_key
    assert_equal "dm:#{Digest::SHA256.hexdigest([ users(:david).id, users(:jz).id ].sort.join(","))}",
      room.direct_member_key
    assert_equal Rooms::Direct.member_key_for([ users(:david).id, users(:jz).id ]),
      Rooms::Direct.member_key_for([ users(:jz).id, users(:david).id ])
  end

  test "lookup by member set runs one indexed query however many DMs exist" do
    room = Rooms::Direct.find_or_create_for([ users(:david), users(:jz) ])
    5.times do |i|
      peer = User.create!(name: "Peer #{i}", email_address: "peer#{i}@example.test")
      Rooms::Direct.find_or_create_for([ users(:david), peer ])
    end

    assert_queries_count 1 do
      assert_equal room, Rooms::Direct.find_for([ users(:jz), users(:david) ])
    end
  end

  test "lookup still finds rooms created before the member key" do
    room = rooms(:david_and_kevin)
    assert_nil room.direct_member_key

    assert_equal room, Rooms::Direct.find_for([ users(:david), users(:kevin) ])
    assert_equal room, Rooms::Direct.find_for([ users(:kevin), users(:david) ])
  end

  test "adding or removing a member changes the member key" do
    room = Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin) ])
    before = room.direct_member_key

    room.add_members([ users(:jz) ], added_by: users(:david))
    assert_not_equal before, room.reload.direct_member_key

    room.leave(users(:jz))
    assert_equal before, room.reload.direct_member_key
  end

  test "a mutated group keeps its own room when its set collides with another group" do
    reused = Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin) ])
    other = Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin), users(:jz) ])

    other.leave(users(:jz))

    assert_equal [ users(:david).id, users(:jason).id, users(:kevin).id ].sort, other.reload.user_ids.sort
    assert_not_equal reused.direct_member_key, other.direct_member_key
    # Only the create path reuses: the same selection opens the keyed room.
    assert_equal reused, Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin) ])
  end

  test "a deleted room never blocks recreating its member set" do
    room = Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin) ])
    room.begin_destroy!

    assert_nil room.reload.direct_member_key
    fresh = Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin) ])
    assert_not_equal room.id, fresh.id
  end

  test "display name favors the custom name, then first names with a remainder" do
    solo = Rooms::Direct.create_for({ creator: users(:david) }, users: [ users(:david) ])
    assert_equal "David", solo.direct_display_name(for_user: users(:david))

    one_to_one = rooms(:david_and_jason)
    assert_equal "Jason", one_to_one.direct_display_name(for_user: users(:david))

    group = Rooms::Direct.create_for({ creator: users(:david) },
      users: [ users(:david), users(:jason), users(:kevin), users(:jz) ])
    assert_equal "Jason, JZ, Kevin", group.direct_display_name(for_user: users(:david))

    fourth = User.create!(name: "Zed Last", email_address: "zed@example.test")
    group.add_members([ fourth ], added_by: users(:david))
    assert_equal "Jason, JZ, Kevin +1", group.direct_display_name(for_user: users(:david))

    group.rename("Weekend Plans", renamed_by: users(:david))
    assert_equal "Weekend Plans", group.direct_display_name(for_user: users(:david))
  end

  test "display name accepts preloaded members without querying" do
    group = Rooms::Direct.create_for({ creator: users(:david) },
      users: [ users(:david), users(:jason), users(:kevin) ])
    members = group.memberships.includes(:user).map(&:user)

    assert_no_queries do
      assert_equal "Jason, Kevin", group.direct_display_name(for_user: users(:david), members: members)
    end
  end

  test "only groups can be renamed or widened" do
    one_to_one = rooms(:david_and_jason)

    assert_raises(Rooms::Direct::NotAGroup) { one_to_one.rename("Sneaky", renamed_by: users(:david)) }
    assert_raises(Rooms::Direct::NotAGroup) { one_to_one.add_members([ users(:kevin) ], added_by: users(:david)) }
    assert_nil one_to_one.reload.name
  end

  test "a named group that shrank to two members keeps its identity" do
    group = Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin) ])
    group.rename("Weekend Plans", renamed_by: users(:david))
    group.leave(users(:kevin))

    assert_equal "Weekend Plans", group.reload.direct_display_name(for_user: users(:david))
    group.rename("Still Plans", renamed_by: users(:david))
    assert_equal "Still Plans", group.reload.name
  end

  test "rename validates length and clearing restores the default name" do
    group = Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin) ])

    assert_raises(ActiveRecord::RecordInvalid) { group.rename("x" * 101, renamed_by: users(:david)) }

    group.rename("  ", renamed_by: users(:david))
    assert_nil group.reload.name
    assert_equal "Jason, Kevin", group.direct_display_name(for_user: users(:david))
  end

  test "add_members enforces the cap and skips existing members" do
    group = Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin) ])

    assert_equal [], group.add_members([ users(:jason) ], added_by: users(:david))

    users = 7.times.map { |i| User.create!(name: "Extra #{i}", email_address: "extra#{i}@example.test") }
    group.add_members(users, added_by: users(:david))
    assert_equal Rooms::Direct::MAX_MEMBERS, group.memberships.count

    overflow = User.create!(name: "Overflow", email_address: "overflow@example.test")
    assert_raises(Rooms::Direct::OverCapacity) { group.add_members([ overflow ], added_by: users(:david)) }
    assert_not group.user_ids.include?(overflow.id)
  end

  test "membership changes post system messages without activity items" do
    group = Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin) ])

    assert_difference -> { group.messages.where(system: true).count }, +1 do
      assert_no_difference -> { ActivityItem.count } do
        group.rename("Weekend Plans", renamed_by: users(:david))
      end
    end
    assert_equal "David renamed the group to Weekend Plans", group.messages.where(system: true).last.plain_text_body

    group.add_members([ users(:jz) ], added_by: users(:david))
    assert_equal "David added JZ to the group", group.messages.where(system: true).last.plain_text_body

    group.leave(users(:jz))
    assert_equal "JZ left the group", group.messages.where(system: true).last.plain_text_body
  end

  test "leaving keeps the group working for everyone left" do
    group = Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin) ])

    assert_equal :left, group.leave(users(:kevin))

    assert_not group.reload.user_ids.include?(users(:kevin).id)
    assert_equal [ users(:david).id, users(:jason).id ].sort, group.user_ids.sort
    assert_predicate group.reload, :persisted?
    assert_nil group.deleted_at
  end

  test "the last member out destroys the room" do
    group = Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin) ])
    group.leave(users(:jason))
    group.leave(users(:kevin))

    assert_equal :destroyed, group.leave(users(:david))
    assert_predicate group.reload, :deleted?
  end
end
