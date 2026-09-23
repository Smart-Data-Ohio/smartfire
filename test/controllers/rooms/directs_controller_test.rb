require "test_helper"

class Rooms::DirectsControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
  end

  test "create" do
    post rooms_directs_url, params: { user_ids: [ users(:jz).id ] }

    room = Room.last
    assert_redirected_to room_url(room)
    assert room.users.include?(users(:david))
    assert room.users.include?(users(:jz))
  end

  test "create only once per user set" do
    assert_difference -> { Room.all.count }, +1 do
      post rooms_directs_url, params: { user_ids: [ users(:jz).id ] }
      post rooms_directs_url, params: { user_ids: [ users(:jz).id ] }
    end
  end

  test "create opens a group DM for several people and reuses it" do
    room = nil

    assert_difference -> { Room.directs.count }, +1 do
      post rooms_directs_url, params: { user_ids: [ users(:jason).id, users(:kevin).id ] }
      room = Room.last
      assert_redirected_to room_url(room)
    end
    assert_equal [ users(:david).id, users(:jason).id, users(:kevin).id ].sort, room.user_ids.sort

    assert_no_difference -> { Room.directs.count } do
      post rooms_directs_url, params: { user_ids: [ users(:kevin).id, users(:jason).id ] }
      assert_redirected_to room_url(room)
    end
  end

  test "create rejects more members than the cap" do
    ids = 10.times.map { |i| User.create!(name: "Cap #{i}", email_address: "cap#{i}@example.test").id }

    assert_no_difference -> { Room.directs.count } do
      post rooms_directs_url, params: { user_ids: ids }
    end

    assert_redirected_to new_rooms_direct_path
    assert_match(/at most/, flash[:alert])
  end

  test "create with start_huddle lands in the room ready to ring" do
    post rooms_directs_url, params: { user_ids: [ users(:jason).id, users(:kevin).id ], start_huddle: "1" }

    assert_redirected_to room_url(Room.last, huddle: "start")
  end

  test "create ignores deactivated and banned users" do
    users(:kevin).update!(status: :deactivated)
    users(:jz).update!(status: :banned)

    assert_no_difference -> { Room.directs.count } do
      post rooms_directs_url, params: { user_ids: [ users(:jason).id, users(:kevin).id, users(:jz).id ] }
    end

    room = Rooms::Direct.find_for([ users(:david), users(:jason) ])
    assert_redirected_to room_url(room)
    assert_equal [ users(:david).id, users(:jason).id ].sort, room.user_ids.sort
  end

  test "create caps user_ids before querying" do
    ids = 50.times.map { |i| User.create!(name: "Many #{i}", email_address: "many#{i}@example.test").id }

    queries = capture_user_queries do
      post rooms_directs_url, params: { user_ids: ids }
    end

    assert_redirected_to new_rooms_direct_path
    assert_capped_id_lists queries
  end

  test "a member can rename the group and everyone sees the system message" do
    room = create_group_dm!([ users(:david), users(:jason), users(:kevin) ])

    patch rooms_direct_url(room), params: { room: { name: "Weekend Plans" } }

    assert_redirected_to edit_rooms_direct_path(room)
    assert_equal "Weekend Plans", room.reload.name
    assert_equal "David renamed the group to Weekend Plans", room.messages.where(system: true).last.plain_text_body

    sign_in :jason
    get room_url(room)
    assert_response :success
    assert_match(/David renamed the group to Weekend Plans/, @response.body)
  end

  test "rename is rejected for one-to-one DMs and by non-members" do
    patch rooms_direct_url(rooms(:david_and_jason)), params: { room: { name: "Sneaky" } }
    assert_redirected_to edit_rooms_direct_path(rooms(:david_and_jason))
    assert_nil rooms(:david_and_jason).reload.name

    room = create_group_dm!([ users(:david), users(:jason), users(:kevin) ])
    sign_in :jz
    patch rooms_direct_url(room), params: { room: { name: "Sneaky" } }
    assert_redirected_to root_url
    assert_nil room.reload.name
  end

  test "a member can add people up to the cap" do
    room = create_group_dm!([ users(:david), users(:jason), users(:kevin) ])

    post add_members_rooms_direct_url(room), params: { user_ids: [ users(:jz).id ] }

    assert_redirected_to edit_rooms_direct_path(room)
    assert room.reload.user_ids.include?(users(:jz).id)
    assert_equal "David added JZ to the group", room.messages.where(system: true).last.plain_text_body
  end

  test "adding members rejects one-to-one DMs, the overflow, and non-members" do
    post add_members_rooms_direct_url(rooms(:david_and_jason)), params: { user_ids: [ users(:kevin).id ] }
    assert_redirected_to edit_rooms_direct_path(rooms(:david_and_jason))
    assert_not rooms(:david_and_jason).reload.user_ids.include?(users(:kevin).id)

    room = create_group_dm!([ users(:david), users(:jason), users(:kevin) ])
    extras = 7.times.map { |i| User.create!(name: "Full #{i}", email_address: "full#{i}@example.test").id }
    room.add_members(User.where(id: extras), added_by: users(:david))
    overflow = User.create!(name: "Overflow", email_address: "overflow2@example.test")

    post add_members_rooms_direct_url(room), params: { user_ids: [ overflow.id ] }
    assert_redirected_to edit_rooms_direct_path(room)
    assert_match(/at most/, flash[:alert])
    assert_not room.reload.user_ids.include?(overflow.id)

    sign_in :jz
    post add_members_rooms_direct_url(room), params: { user_ids: [ users(:kevin).id ] }
    assert_redirected_to root_url
  end

  test "adding members caps user_ids before querying" do
    room = create_group_dm!([ users(:david), users(:jason), users(:kevin) ])
    ids = 50.times.map { |i| User.create!(name: "Extra #{i}", email_address: "addmany#{i}@example.test").id }

    queries = capture_user_queries do
      post add_members_rooms_direct_url(room), params: { user_ids: ids }
    end

    assert_redirected_to edit_rooms_direct_path(room)
    assert_match(/at most/, flash[:alert])
    assert_capped_id_lists queries
  end

  test "leaving removes only your membership and the group keeps working" do
    room = create_group_dm!([ users(:david), users(:jason), users(:kevin) ])

    delete leave_rooms_direct_url(room)

    assert_redirected_to root_url
    assert_not room.reload.user_ids.include?(users(:david).id)
    assert_equal "David left the group", room.messages.where(system: true).last.plain_text_body

    # The leaver lost access while the others keep the room.
    get room_url(room)
    assert_redirected_to root_url

    sign_in :jason
    get room_url(room)
    assert_response :success
  end

  test "the last member out destroys the group" do
    sign_in :jason
    room = create_group_dm!([ users(:jason) ])

    assert_enqueued_with(job: Room::DestroyJob, args: [ room.id ]) do
      delete leave_rooms_direct_url(room)
      assert_redirected_to root_url
    end

    assert_predicate room.reload, :deleted?
  end

  test "leave is rejected for non-members" do
    room = create_group_dm!([ users(:david), users(:jason), users(:kevin) ])

    sign_in :jz
    delete leave_rooms_direct_url(room)

    assert_redirected_to root_url
    assert_equal 3, room.reload.memberships.count
  end

  test "a non-member cannot read the group" do
    room = create_group_dm!([ users(:jason), users(:kevin), users(:jz) ])

    get room_url(room)

    assert_redirected_to root_url
  end

  test "a removed member loses access to the group" do
    room = create_group_dm!([ users(:david), users(:jason), users(:kevin) ])
    room.leave(users(:david))

    get room_url(room)

    assert_redirected_to root_url
  end

  test "destroy only allowed for all room users" do
    sign_in :kevin
    room = rooms(:david_and_kevin)

    assert_enqueued_with(job: Room::DestroyJob, args: [ room.id ]) do
      delete rooms_direct_url(room)
      assert_redirected_to root_url
    end

    assert_predicate room.reload, :deleted?
    assert_empty room.memberships

    assert_difference -> { Room.count }, -1 do
      perform_enqueued_jobs
    end
  end

  test "destroy can't reach a closed room the member didn't create" do
    sign_in :kevin

    assert_no_difference -> { Room.count } do
      delete rooms_direct_url(rooms(:designers))
    end

    assert rooms(:designers).reload.persisted?
  end

  test "destroy can't reach an open room the member didn't create" do
    sign_in :kevin

    assert_no_difference -> { Room.count } do
      delete rooms_direct_url(rooms(:hq))
    end

    assert rooms(:hq).reload.persisted?
  end

  test "destroy can't reach a room the member isn't in at all" do
    sign_in :jz

    assert_no_difference -> { Room.count } do
      delete rooms_direct_url(rooms(:david_and_kevin))
    end

    assert rooms(:david_and_kevin).reload.persisted?
  end

  private
    def create_group_dm!(*members)
      members = members.flatten
      Current.set(user: members.first) { Rooms::Direct.find_or_create_for(members) }
    end

    # Every IN-list lookup stays within the capped submission plus the
    # current user, who joins the list after the cap.
    def assert_capped_id_lists(queries)
      lists = queries.filter_map { |sql| sql[/IN \(([^)]*)\)/, 1] }

      assert_not_empty lists, "expected at least one capped users lookup"
      lists.each do |list|
        assert_operator list.count("?"), :<=, Rooms::Direct::MAX_MEMBERS + 1, "expected a capped id list, got: #{list}"
      end
    end

    # Every users-table statement the block runs, except query-cache hits.
    def capture_user_queries(&block)
      queries = []
      subscription = ActiveSupport::Notifications.subscribe("sql.active_record") do |*, payload|
        queries << payload[:sql] if !payload[:cached] && payload[:sql].include?('FROM "users"')
      end

      ActiveRecord::Base.connection.clear_query_cache
      block.call
      queries
    ensure
      ActiveSupport::Notifications.unsubscribe(subscription)
    end
end
