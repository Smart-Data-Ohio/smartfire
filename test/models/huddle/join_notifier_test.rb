require "test_helper"

class Huddle::JoinNotifierTest < ActiveSupport::TestCase
  include ActiveJob::TestHelper

  setup do
    @original_api_secret = ENV["LIVEKIT_API_SECRET"]
    ENV["LIVEKIT_API_SECRET"] = "test-api-secret"
    @room = rooms(:david_and_jason)
    @pool = Rails.configuration.x.web_push_pool
  end

  teardown do
    ENV["LIVEKIT_API_SECRET"] = @original_api_secret
  end

  test "an in-call DM member is told when the peer joins, and the joiner is not" do
    david_grant = issue_seen(@room, users(:david), memberships(:david_david_and_jason))
    issue_seen(@room, users(:jason), memberships(:jason_david_and_jason))

    @pool.expects(:queue).never
    Huddle::JoinNotifier.notify_join(david_grant)

    assert_broadcast_on(notice_stream(users(:jason)), {
      huddleJoinNotice: {
        eventType: "huddle_joined",
        roomId: @room.id,
        roomName: "David",
        roomPath: Rails.application.routes.url_helpers.room_path(@room),
        joinerId: users(:david).id,
        joinerName: "David",
        inCall: true,
        rejoin: false
      }
    })
    assert_broadcasts notice_stream(users(:david)), 0
  end

  test "a member with no access to the room is told nothing" do
    david_grant = issue_seen(@room, users(:david), memberships(:david_david_and_jason))
    resolve_ring(users(:jason))

    @pool.expects(:queue).once
    Huddle::JoinNotifier.notify_join(david_grant)

    assert_broadcasts notice_stream(users(:kevin)), 0
  end

  test "an out-of-call DM member gets the banner broadcast and one push" do
    david_grant = issue_seen(@room, users(:david), memberships(:david_david_and_jason))
    resolve_ring(users(:jason))

    @pool.expects(:queue).once.with do |payload, subscriptions|
      assert_equal "David joined your huddle", payload[:title]
      assert_equal Rails.application.routes.url_helpers.room_path(@room), payload[:path]
      assert_equal "huddle-#{@room.id}", payload[:tag]
      assert_equal [ users(:jason).id ], subscriptions.order(:id).pluck(:user_id)
      true
    end
    Huddle::JoinNotifier.notify_join(david_grant)

    assert_broadcast_on(notice_stream(users(:jason)), {
      huddleJoinNotice: {
        eventType: "huddle_joined",
        roomId: @room.id,
        roomName: "David",
        roomPath: Rails.application.routes.url_helpers.room_path(@room),
        joinerId: users(:david).id,
        joinerName: "David",
        inCall: false,
        rejoin: false
      }
    })
  end

  test "a group DM join toasts the insider and banners the outsider" do
    group = Rooms::Direct.create_for({ creator: users(:david) }, users: [ users(:david), users(:jason), users(:kevin) ])
    david_grant = issue_seen(group, users(:david), group.memberships.find_by!(user: users(:david)))
    issue_seen(group, users(:jason), group.memberships.find_by!(user: users(:jason)))
    resolve_ring(users(:kevin))

    @pool.expects(:queue).once.with do |_payload, subscriptions|
      assert_equal [ users(:kevin).id ], subscriptions.order(:id).pluck(:user_id)
      true
    end
    Huddle::JoinNotifier.notify_join(david_grant)

    assert_broadcast_on(notice_stream(users(:jason)), {
      huddleJoinNotice: {
        eventType: "huddle_joined",
        roomId: group.id,
        roomName: group.direct_display_name(for_user: users(:jason)),
        roomPath: Rails.application.routes.url_helpers.room_path(group),
        joinerId: users(:david).id,
        joinerName: "David",
        inCall: true,
        rejoin: false
      }
    })
    assert_broadcast_on(notice_stream(users(:kevin)), {
      huddleJoinNotice: {
        eventType: "huddle_joined",
        roomId: group.id,
        roomName: group.direct_display_name(for_user: users(:kevin)),
        roomPath: Rails.application.routes.url_helpers.room_path(group),
        joinerId: users(:david).id,
        joinerName: "David",
        inCall: false,
        rejoin: false
      }
    })
  end

  test "join fan-out loads members and rings once no matter the group size" do
    group = Rooms::Direct.create_for({ creator: users(:david) },
      users: [ users(:david), users(:jason), users(:kevin), users(:jz) ])
    david_grant = issue_seen(group, users(:david), group.memberships.find_by!(user: users(:david)))
    issue_seen(group, users(:jason), group.memberships.find_by!(user: users(:jason)))
    resolve_ring(users(:kevin))
    resolve_ring(users(:jz))
    david_grant.user # warm the joiner outside the counted block

    @pool.stubs(:queue)
    user_selects, ring_queries = count_join_queries { Huddle::JoinNotifier.notify_join(david_grant) }

    # The member preload plus one shared member list (not one member list
    # per viewer), and one ring lookup for the room (not one per outsider).
    assert_equal 2, user_selects.size
    assert_equal 1, ring_queries.size
  end

  test "a channel join toasts the insider and tells the outsider nothing" do
    room = rooms(:watercooler)
    david_grant = issue_seen(room, users(:david), memberships(:david_watercooler))
    issue_seen(room, users(:jason), memberships(:jason_watercooler))

    @pool.expects(:queue).never
    Huddle::JoinNotifier.notify_join(david_grant)

    assert_broadcast_on(notice_stream(users(:jason)), {
      huddleJoinNotice: {
        eventType: "huddle_joined",
        roomId: room.id,
        roomName: room.name,
        roomPath: Rails.application.routes.url_helpers.room_path(room),
        joinerId: users(:david).id,
        joinerName: "David",
        inCall: true,
        rejoin: false
      }
    })
    assert_broadcasts notice_stream(users(:bender)), 0
  end

  test "an out-of-call channel member gets no banner and no push" do
    room = rooms(:watercooler)
    david_grant = issue_seen(room, users(:david), memberships(:david_watercooler))

    @pool.expects(:queue).never
    assert_broadcasts(notice_stream(users(:jason)), 0) do
      Huddle::JoinNotifier.notify_join(david_grant)
    end
  end

  test "a voice room join toasts the insider and tells the outsider nothing" do
    room = Rooms::Voice.create_for({ name: "Lounge", creator: users(:david) }, users: [ users(:david), users(:jason) ])
    david_grant = issue_seen(room, users(:david), room.memberships.find_by!(user: users(:david)))

    @pool.expects(:queue).never
    assert_broadcasts(notice_stream(users(:jason)), 0) do
      Huddle::JoinNotifier.notify_join(david_grant)
    end

    issue_seen(room, users(:jason), room.memberships.find_by!(user: users(:jason)))
    Huddle::JoinNotifier.notify_join(david_grant)

    assert_broadcast_on(notice_stream(users(:jason)), {
      huddleJoinNotice: {
        eventType: "huddle_joined",
        roomId: room.id,
        roomName: room.name,
        roomPath: Rails.application.routes.url_helpers.room_path(room),
        joinerId: users(:david).id,
        joinerName: "David",
        inCall: true,
        rejoin: false
      }
    })
  end

  test "bots and deactivated members are told nothing" do
    group = Rooms::Direct.create_for({ creator: users(:david) },
      users: [ users(:david), users(:jason), users(:bender) ])
    users(:jason).update!(status: :deactivated)
    david_grant = issue_seen(group, users(:david), group.memberships.find_by!(user: users(:david)))

    @pool.expects(:queue).never
    assert_broadcasts(notice_stream(users(:bender)), 0) do
      assert_broadcasts(notice_stream(users(:jason)), 0) do
        Huddle::JoinNotifier.notify_join(david_grant)
      end
    end
  end

  test "a bot join notifies nobody" do
    grant = HuddleGrant.new(room: @room, user: users(:bender))

    @pool.expects(:queue).never
    assert_broadcasts(notice_stream(users(:jason)), 0) do
      assert_broadcasts(notice_stream(users(:david)), 0) do
        Huddle::JoinNotifier.notify_join(grant)
      end
    end
  end

  test "a second device sighted while the first is listed enqueues no join notice" do
    issue_seen(@room, users(:david), memberships(:david_david_and_jason))
    second_grant = HuddleGrant.issue!(session: second_session_for(users(:david)),
      membership: memberships(:david_david_and_jason))

    assert_no_enqueued_jobs only: Huddle::JoinNoticeJob do
      second_grant.record_seen!
    end
  end

  test "sightings from two devices before the job runs still notify once" do
    first_grant = HuddleGrant.issue!(session: sessions_for(users(:david)),
      membership: memberships(:david_david_and_jason))
    second_grant = HuddleGrant.issue!(session: second_session_for(users(:david)),
      membership: memberships(:david_david_and_jason))
    resolve_ring(users(:jason))

    # Both devices connect before either job runs. The first sighting saw
    # no other listing, so its job notifies; the second sighting saw the
    # first listed, so it enqueues nothing.
    first_grant.record_seen!
    second_grant.record_seen!

    @pool.expects(:queue).once
    assert_broadcasts(notice_stream(users(:jason)), 1) do
      perform_enqueued_jobs only: Huddle::JoinNoticeJob
    end
  end

  test "a join job running after the joiner left notifies nobody" do
    grant = HuddleGrant.issue!(session: sessions(:david_safari),
      membership: memberships(:david_david_and_jason))
    grant.record_seen!
    resolve_ring(users(:jason))
    assert grant.mark_out_of_call!

    @pool.expects(:queue).never
    assert_broadcasts(notice_stream(users(:jason)), 0) do
      perform_enqueued_jobs only: Huddle::JoinNoticeJob
    end
  end

  test "a viewer whose ring is still live gets no join notice" do
    david_grant = issue_seen(@room, users(:david), memberships(:david_david_and_jason))

    @pool.expects(:queue).never
    assert_broadcasts(notice_stream(users(:jason)), 0) do
      Huddle::JoinNotifier.notify_join(david_grant)
    end
  end

  test "a viewer whose ring went stale gets the join notice again" do
    david_grant = issue_seen(@room, users(:david), memberships(:david_david_and_jason))

    travel 61.seconds do
      david_grant.update_columns(last_seen_at: Time.current) # still connected
      @pool.expects(:queue).once
      assert_broadcasts(notice_stream(users(:jason)), 1) do
        Huddle::JoinNotifier.notify_join(david_grant)
      end
    end
  end

  test "a switched-off or hidden room stays silent for the outsider" do
    david_grant = issue_seen(@room, users(:david), memberships(:david_david_and_jason))
    resolve_ring(users(:jason))

    memberships(:jason_david_and_jason).update!(involvement: "nothing")

    @pool.expects(:queue).never
    assert_broadcasts(notice_stream(users(:jason)), 0) do
      Huddle::JoinNotifier.notify_join(david_grant)
    end

    memberships(:jason_david_and_jason).update!(involvement: "invisible")

    assert_broadcasts(notice_stream(users(:jason)), 0) do
      Huddle::JoinNotifier.notify_join(david_grant)
    end
  end

  test "an outsider with huddle invitations switched off still banners but gets no push" do
    david_grant = issue_seen(@room, users(:david), memberships(:david_david_and_jason))
    resolve_ring(users(:jason))
    users(:jason).update!(inbox_preferences: { "huddle_invitations" => false })

    @pool.expects(:queue).never
    assert_broadcasts(notice_stream(users(:jason)), 1) do
      Huddle::JoinNotifier.notify_join(david_grant)
    end
  end

  test "a muted room still banners the outsider but sends no push" do
    david_grant = issue_seen(@room, users(:david), memberships(:david_david_and_jason))
    resolve_ring(users(:jason))
    memberships(:jason_david_and_jason).update!(involvement: "muted")

    @pool.expects(:queue).never
    assert_broadcasts(notice_stream(users(:jason)), 1) do
      Huddle::JoinNotifier.notify_join(david_grant)
    end
  end

  test "a join after a recent in-call revoke is marked as a rejoin" do
    david_grant = issue_seen(@room, users(:david), memberships(:david_david_and_jason))
    issue_seen(@room, users(:jason), memberships(:jason_david_and_jason))
    david_grant.revoke!

    rejoined = issue_seen(@room, users(:david), memberships(:david_david_and_jason))

    @pool.expects(:queue).never
    Huddle::JoinNotifier.notify_join(rejoined)

    assert_broadcast_on(notice_stream(users(:jason)), {
      huddleJoinNotice: {
        eventType: "huddle_joined",
        roomId: @room.id,
        roomName: "David",
        roomPath: Rails.application.routes.url_helpers.room_path(@room),
        joinerId: users(:david).id,
        joinerName: "David",
        inCall: true,
        rejoin: true
      }
    })
  end

  test "a rejoin after the disconnect report cleared liveness is not marked" do
    david_grant = issue_seen(@room, users(:david), memberships(:david_david_and_jason))
    issue_seen(@room, users(:jason), memberships(:jason_david_and_jason))
    david_grant.revoke!
    # The gateway's disconnect report only posts after the three-second
    # reconnect grace, so a join sighted after it follows its leave by
    # seconds: no delivery race can flip that order, and the client's
    # pending leave swallows the join without any mark.
    travel 1.second do
      assert david_grant.mark_out_of_call!
    end

    rejoined = issue_seen(@room, users(:david), memberships(:david_david_and_jason))

    @pool.expects(:queue).never
    Huddle::JoinNotifier.notify_join(rejoined)

    assert_broadcast_on(notice_stream(users(:jason)), {
      huddleJoinNotice: {
        eventType: "huddle_joined",
        roomId: @room.id,
        roomName: "David",
        roomPath: Rails.application.routes.url_helpers.room_path(@room),
        joinerId: users(:david).id,
        joinerName: "David",
        inCall: true,
        rejoin: false
      }
    })
  end

  test "a join after a quiet revoke is not marked as a rejoin" do
    david_grant = HuddleGrant.issue!(session: sessions_for(users(:david)),
      membership: memberships(:david_david_and_jason))
    issue_seen(@room, users(:jason), memberships(:jason_david_and_jason))
    david_grant.revoke!

    rejoined = issue_seen(@room, users(:david), memberships(:david_david_and_jason))

    @pool.expects(:queue).never
    Huddle::JoinNotifier.notify_join(rejoined)

    assert_broadcast_on(notice_stream(users(:jason)), {
      huddleJoinNotice: {
        eventType: "huddle_joined",
        roomId: @room.id,
        roomName: "David",
        roomPath: Rails.application.routes.url_helpers.room_path(@room),
        joinerId: users(:david).id,
        joinerName: "David",
        inCall: true,
        rejoin: false
      }
    })
  end

  test "a join after the rejoin window is not marked as a rejoin" do
    david_grant = issue_seen(@room, users(:david), memberships(:david_david_and_jason))
    david_grant.revoke!

    travel(Huddle::JoinNotifier::REJOIN_WINDOW + 1.second) do
      issue_seen(@room, users(:jason), memberships(:jason_david_and_jason))
      rejoined = issue_seen(@room, users(:david), memberships(:david_david_and_jason))

      @pool.expects(:queue).never
      Huddle::JoinNotifier.notify_join(rejoined)

      assert_broadcast_on(notice_stream(users(:jason)), {
        huddleJoinNotice: {
          eventType: "huddle_joined",
          roomId: @room.id,
          roomName: "David",
          roomPath: Rails.application.routes.url_helpers.room_path(@room),
          joinerId: users(:david).id,
          joinerName: "David",
          inCall: true,
          rejoin: false
        }
      })
    end
  end

  test "leaving toasts the members still in the call" do
    david_grant = issue_seen(@room, users(:david), memberships(:david_david_and_jason))
    issue_seen(@room, users(:jason), memberships(:jason_david_and_jason))

    assert david_grant.mark_out_of_call!

    assert_broadcast_on(notice_stream(users(:jason)), {
      huddleJoinNotice: {
        eventType: "huddle_left",
        roomId: @room.id,
        roomName: "David",
        joinerId: users(:david).id,
        joinerName: "David"
      }
    })
    assert_broadcasts notice_stream(users(:david)), 0
  end

  test "revoking an in-call grant toasts the members still in the call" do
    david_grant = issue_seen(@room, users(:david), memberships(:david_david_and_jason))
    issue_seen(@room, users(:jason), memberships(:jason_david_and_jason))

    david_grant.revoke!

    assert_broadcast_on(notice_stream(users(:jason)), {
      huddleJoinNotice: {
        eventType: "huddle_left",
        roomId: @room.id,
        roomName: "David",
        joinerId: users(:david).id,
        joinerName: "David"
      }
    })
  end

  test "leaving tells out-of-call DM members so their banner drops the leaver" do
    group = Rooms::Direct.create_for({ creator: users(:david) }, users: [ users(:david), users(:jason), users(:kevin) ])
    david_grant = issue_seen(group, users(:david), group.memberships.find_by!(user: users(:david)))
    issue_seen(group, users(:jason), group.memberships.find_by!(user: users(:jason)))

    assert david_grant.mark_out_of_call!

    assert_broadcast_on(notice_stream(users(:jason)), {
      huddleJoinNotice: {
        eventType: "huddle_left",
        roomId: group.id,
        roomName: group.direct_display_name(for_user: users(:jason)),
        joinerId: users(:david).id,
        joinerName: "David"
      }
    })
    assert_broadcast_on(notice_stream(users(:kevin)), {
      huddleJoinNotice: {
        eventType: "huddle_left",
        roomId: group.id,
        roomName: group.direct_display_name(for_user: users(:kevin)),
        joinerId: users(:david).id,
        joinerName: "David"
      }
    })
  end

  test "leaving a channel tells out-of-call members nothing" do
    room = rooms(:watercooler)
    Membership.create!(room: room, user: users(:kevin))
    david_grant = issue_seen(room, users(:david), memberships(:david_watercooler))
    issue_seen(room, users(:jason), memberships(:jason_watercooler))

    assert_broadcasts(notice_stream(users(:kevin)), 0) do
      assert david_grant.mark_out_of_call!
    end
  end

  test "the last one out of a DM dismisses every other member's banner" do
    david_grant = issue_seen(@room, users(:david), memberships(:david_david_and_jason))

    assert_broadcasts(notice_stream(users(:jason)), 1) do
      assert david_grant.mark_out_of_call!
    end

    assert_broadcast_on(notice_stream(users(:jason)), {
      huddleJoinNotice: { eventType: "huddle_ended", roomId: @room.id }
    })
  end

  test "the last one out of a channel dismisses nothing" do
    room = rooms(:watercooler)
    david_grant = issue_seen(room, users(:david), memberships(:david_watercooler))

    assert_broadcasts(notice_stream(users(:jason)), 0) do
      assert david_grant.mark_out_of_call!
    end
  end

  test "leaving a call the grant was never in toasts nobody" do
    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))

    assert_broadcasts(notice_stream(users(:jason)), 0) do
      assert_not grant.mark_out_of_call!
    end
  end

  test "revoking a quiet grant toasts nobody" do
    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))

    assert_broadcasts(notice_stream(users(:jason)), 0) do
      grant.revoke!
    end
  end

  test "leaving while another of the leaver's grants is still in toasts nobody" do
    issue_seen(@room, users(:david), memberships(:david_david_and_jason))
    issue_seen(@room, users(:jason), memberships(:jason_david_and_jason))
    second_grant = issue_seen(@room, users(:david), memberships(:david_david_and_jason), session: second_session_for(users(:david)))

    assert_broadcasts(notice_stream(users(:jason)), 0) do
      assert second_grant.mark_out_of_call!
    end
  end

  test "a leave report after a revocation does not toast twice" do
    david_grant = issue_seen(@room, users(:david), memberships(:david_david_and_jason))
    issue_seen(@room, users(:jason), memberships(:jason_david_and_jason))
    david_grant.revoke!

    assert_broadcasts(notice_stream(users(:jason)), 0) do
      assert david_grant.mark_out_of_call!
    end
  end

  private
    def notice_stream(user)
      HuddleNoticeChannel.stream_name_for(user.id)
    end

    def issue_seen(room, user, membership, session: nil)
      grant = HuddleGrant.issue!(session: session || sessions_for(user), membership: membership)
      grant.update_columns(last_seen_at: Time.current)
      grant
    end

    def sessions_for(user)
      Session.create!(user: user, user_agent: "join notice test", ip_address: "127.0.0.9")
    end

    def second_session_for(user)
      Session.create!(user: user, user_agent: "second device", ip_address: "127.0.0.2")
    end

    # The start-of-huddle ring resolved to a missed call, so a later join
    # notice is no longer shadowed by a live ring.
    def resolve_ring(user)
      ActivityItem.where(user: user, event_type: "huddle_started").update_all(event_type: "huddle_missed")
    end

    # Same shape as the count_queries in ChannelThreadsControllerTest:
    # every uncached users SELECT and every activity_items query in the
    # block, so the fan-out can prove it preloads instead of querying
    # per viewer.
    def count_join_queries
      user_selects = []
      ring_queries = []
      subscription = ActiveSupport::Notifications.subscribe("sql.active_record") do |*, payload|
        next if payload[:cached]

        sql = payload[:sql].to_s
        user_selects << sql if sql.match?(/FROM "users"/)
        ring_queries << sql if sql.match?(/FROM "activity_items"/)
      end

      ActiveRecord::Base.connection.clear_query_cache
      yield
      [ user_selects, ring_queries ]
    ensure
      ActiveSupport::Notifications.unsubscribe(subscription)
    end
end
