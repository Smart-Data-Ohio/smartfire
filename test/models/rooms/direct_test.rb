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

  test "membership changes post system notes without activity items" do
    group = Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin) ])

    assert_difference -> { group.messages.where(system_note: true).count }, +1 do
      assert_no_difference -> { ActivityItem.count } do
        group.rename("Weekend Plans", renamed_by: users(:david))
      end
    end
    assert_equal "renamed the group to Weekend Plans", group.messages.where(system_note: true).last.plain_text_body

    group.add_members([ users(:jz) ], added_by: users(:david))
    assert_equal "added JZ to the group", group.messages.where(system_note: true).last.plain_text_body

    group.leave(users(:jz))
    assert_equal "left the group", group.messages.where(system_note: true).last.plain_text_body
  end

  test "rename notes mark nobody unread and enqueue no push" do
    group = Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin) ])

    assert_no_enqueued_jobs(only: Room::PushMessageJob) do
      group.rename("Weekend Plans", renamed_by: users(:david))
    end

    assert_empty group.memberships.unread
  end

  test "rename notes are never delivered to agents" do
    group = Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:bender) ])
    assert users(:bender).agent.present?

    assert_no_difference -> { AgentEvent.count } do
      assert_no_enqueued_jobs(only: Agent::DeliveryJob) do
        group.rename("Weekend Plans", renamed_by: users(:david))
      end
    end
  end

  test "rename notes stay out of the search index" do
    group = Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin) ])

    group.rename("Zymurgy Plans", renamed_by: users(:david))

    assert_predicate group.messages.where(system_note: true).last, :present?
    assert_empty Message.search("Zymurgy")
  end

  test "rename notes broadcast into the timeline without unread" do
    group = Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin) ])

    rename = -> { group.rename("Weekend Plans", renamed_by: users(:david)) }
    streams = group.users.map { |member| UnreadRoomsChannel.stream_name_for(member.id) }
    quiet = streams.reverse.reduce(rename) { |block, stream| -> { assert_no_broadcasts(stream, &block) } }

    assert_broadcasts room_messages_stream_name(group), 1, &quiet
  end

  test "rename notes are rate-limited to one per room per minute" do
    group = Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin) ])

    group.rename("First Name", renamed_by: users(:david))
    group.rename("Second Name", renamed_by: users(:david))

    assert_equal "Second Name", group.reload.name
    assert_equal 1, group.messages.where(system_note: true).count

    travel 61.seconds do
      group.rename("Third Name", renamed_by: users(:david))
    end

    assert_equal "Third Name", group.reload.name
    assert_equal 2, group.messages.where(system_note: true).count
  end

  test "adding members broadcasts a sidebar row to newcomers and refreshes the rest" do
    group = Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin) ])

    member_broadcasts = capture_broadcasts(user_rooms_stream_name(users(:jason))) do
      newcomer_broadcasts = capture_broadcasts(user_rooms_stream_name(users(:jz))) do
        # Explicit locals, not request state: bender is outside the room,
        # so the row must still render each recipient's own view.
        Current.set(user: users(:bender)) do
          group.add_members([ users(:jz) ], added_by: users(:david))
        end
      end

      # The newcomer gets a prepended row plus a header.
      assert_equal 2, newcomer_broadcasts.size
      assert_includes newcomer_broadcasts.map(&:to_s).join, "David, Jason, Kevin"
    end

    # Existing members get a replaced row plus a header.
    assert_equal 2, member_broadcasts.size
  end

  test "renaming broadcasts every member's sidebar row and room header" do
    group = Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin) ])

    member_broadcasts = capture_broadcasts(user_rooms_stream_name(users(:jason))) do
      group.rename("Weekend Plans", renamed_by: users(:david))
    end

    assert_equal 2, member_broadcasts.size
    assert_includes member_broadcasts.map(&:to_s).join, "Weekend Plans"
  end

  test "header broadcasts render each member's own default name" do
    group = Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin) ])
    group.rename("Weekend Plans", renamed_by: users(:david))

    member_broadcasts = capture_broadcasts(user_rooms_stream_name(users(:jason))) do
      group.rename("", renamed_by: users(:david))
    end

    header = member_broadcasts.map(&:to_s).find { |html| html.include?(ActionView::RecordIdentifier.dom_id(group, :header)) }
    assert header, "expected a header replace"
    assert_includes header, "David, Kevin"
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

  private
    def room_messages_stream_name(room)
      signed = Turbo::StreamsChannel.signed_stream_name([ room, :messages ])
      Turbo::StreamsChannel.verified_stream_name(signed)
    end

    def user_rooms_stream_name(user)
      signed = Turbo::StreamsChannel.signed_stream_name([ user, :rooms ])
      Turbo::StreamsChannel.verified_stream_name(signed)
    end
end
