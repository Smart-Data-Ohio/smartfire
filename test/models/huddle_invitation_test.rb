require "test_helper"

class HuddleInvitationTest < ActiveSupport::TestCase
  include ActiveJob::TestHelper

  setup do
    @original_api_secret = ENV["LIVEKIT_API_SECRET"]
    ENV["LIVEKIT_API_SECRET"] = "test-api-secret"
    @room = rooms(:david_and_jason)
    @starter_membership = memberships(:david_david_and_jason)
    @starter_session = sessions(:david_safari)
  end

  teardown do
    ENV["LIVEKIT_API_SECRET"] = @original_api_secret
  end

  test "issuing a grant in a one-to-one DM invites only the other participant" do
    assert_enqueued_with(job: Huddle::PushInvitationJob) do
      HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
    end

    item = ActivityItem.find_by!(user: users(:jason), event_type: "huddle_started")
    assert_equal HuddleGrant.polymorphic_name, item.source_type
    assert_equal @room.id, item.source.room_id
    assert_equal users(:david).id, item.source.user_id
    assert_predicate item, :unread?
    assert_not ActivityItem.exists?(user: users(:david), event_type: "huddle_started")
  end

  test "issuing a grant never schedules a delayed job" do
    # Invitations must only enqueue immediate jobs; overdue resolution runs
    # in the reconciler loop instead. Delayed jobs exist for retries, but
    # issuing a grant must never schedule one.
    ActiveJob::Base.queue_adapter.stubs(:enqueue_at).raises(NotImplementedError)

    HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)

    assert ActivityItem.exists?(user: users(:jason), event_type: "huddle_started")
  end

  test "a recipient with notifications off or invisible gets no invitation" do
    memberships(:jason_david_and_jason).update!(involvement: "nothing")

    assert_no_difference -> { ActivityItem.where(user: users(:jason)).count } do
      assert_no_enqueued_jobs only: Huddle::PushInvitationJob do
        HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
      end
    end

    memberships(:jason_david_and_jason).update!(involvement: "invisible")

    assert_no_difference -> { ActivityItem.where(user: users(:jason)).count } do
      HuddleGrant.issue!(session: second_session_for(users(:david)), membership: @starter_membership)
    end
  end

  test "a recipient with huddle items switched off still gets the banner but no item" do
    users(:jason).update!(inbox_preferences: { "huddle_invitations" => false })

    assert_no_difference -> { ActivityItem.where(user: users(:jason)).count } do
      assert_no_enqueued_jobs only: Huddle::PushInvitationJob do
        assert_broadcasts ActivityChannel.stream_name_for(users(:jason).id), 1 do
          HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
        end
      end
    end

    assert_broadcast_on(ActivityChannel.stream_name_for(users(:jason).id), {
      activityItemId: 0,
      huddleInvitation: {
        activityItemId: 0,
        eventType: "huddle_started",
        state: "unread",
        roomId: @room.id,
        roomName: "David",
        roomPath: Rails.application.routes.url_helpers.room_path(@room),
        callerName: "David",
        readPath: "",
        handledPath: ""
      }
    })

    travel 46.seconds do
      Huddle::InvitationResolver.resolve_overdue!
    end
    assert_not ActivityItem.exists?(user: users(:jason))

    mention = rooms(:designers).messages.create!(
      creator: users(:david),
      body: "Hey #{mention_attachment_for(:jason)}",
      client_message_id: "huddle-switch-neighbour"
    )
    assert_equal "mention", ActivityItem.find_by!(user: users(:jason), source: mention).event_type
  end

  test "a switched-off user hears one banner across reissues inside the window and a fresh one after" do
    users(:jason).update!(inbox_preferences: { "huddle_invitations" => false })

    assert_broadcasts ActivityChannel.stream_name_for(users(:jason).id), 1 do
      HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
      HuddleGrant.issue!(session: second_session_for(users(:david)), membership: @starter_membership)
      HuddleGrant.issue!(session: second_session_for(users(:david)), membership: @starter_membership)
    end
    assert_not ActivityItem.exists?(user: users(:jason))

    travel 3.minutes do
      assert_broadcasts ActivityChannel.stream_name_for(users(:jason).id), 1 do
        HuddleGrant.issue!(session: second_session_for(users(:david)), membership: @starter_membership)
      end
    end
    assert_not ActivityItem.exists?(user: users(:jason))
  end

  test "a switched-off user is not rung again when the same session reissues its grant" do
    users(:jason).update!(inbox_preferences: { "huddle_invitations" => false })

    assert_broadcasts ActivityChannel.stream_name_for(users(:jason).id), 1 do
      HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
      HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
    end
  end

  test "channel huddles create no invitation" do
    assert_no_difference -> { ActivityItem.count } do
      assert_no_enqueued_jobs do
        HuddleGrant.issue!(session: @starter_session, membership: memberships(:david_watercooler))
      end
    end
  end

  test "voice channel huddles create no invitation" do
    room = Rooms::Voice.create_for({ name: "Lounge", creator: users(:david) }, users: [ users(:david), users(:jason) ])

    assert_no_difference -> { ActivityItem.count } do
      assert_no_enqueued_jobs do
        HuddleGrant.issue!(session: @starter_session, membership: room.memberships.find_by!(user: users(:david)))
      end
    end
  end

  test "no invitation while the other participant is in the call" do
    recipient_grant = HuddleGrant.issue!(session: second_session_for(users(:jason)), membership: memberships(:jason_david_and_jason))
    recipient_grant.update_columns(last_seen_at: Time.current)

    assert_no_difference -> { ActivityItem.where(user: users(:jason)).count } do
      assert_no_enqueued_jobs do
        HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
      end
    end
  end

  test "an invitation fires when the other participant's grant went quiet" do
    recipient_grant = HuddleGrant.issue!(session: second_session_for(users(:jason)), membership: memberships(:jason_david_and_jason))
    recipient_grant.update_columns(last_seen_at: 21.seconds.ago)

    assert_difference -> { ActivityItem.where(user: users(:jason), event_type: "huddle_started").count }, 1 do
      HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
    end
  end

  test "reusing the same grant rings again once the dedup window has passed" do
    grant = HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
    first_item = ActivityItem.find_by!(user: users(:jason), source: grant)

    travel 3.minutes do
      assert_broadcasts ActivityChannel.stream_name_for(users(:jason).id), 1 do
        assert_enqueued_with(job: Huddle::PushInvitationJob) do
          assert_equal grant, HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
        end
      end
    end

    assert_equal 1, ActivityItem.where(user: users(:jason)).count
    first_item.reload
    assert_equal "huddle_started", first_item.event_type
    assert_predicate first_item, :unread?
    assert_operator first_item.created_at, :>, 1.minute.ago
  end

  test "a second grant for the same starter does not ring again inside two minutes" do
    HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
    assert_equal 1, ActivityItem.where(user: users(:jason), event_type: "huddle_started").count

    assert_no_difference -> { ActivityItem.where(user: users(:jason), event_type: "huddle_started").count } do
      assert_no_enqueued_jobs do
        HuddleGrant.issue!(session: second_session_for(users(:david)), membership: @starter_membership)
      end
    end
  end

  test "no ring inside the two-minute window after a missed invitation" do
    HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
    ActivityItem.find_by!(user: users(:jason)).update!(event_type: "huddle_missed")

    assert_no_difference -> { ActivityItem.where(user: users(:jason)).count } do
      assert_no_enqueued_jobs do
        HuddleGrant.issue!(session: second_session_for(users(:david)), membership: @starter_membership)
      end
    end
  end

  test "a handled invitation still suppresses the next ring inside two minutes" do
    HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
    item = ActivityItem.find_by!(user: users(:jason)).mark_handled!

    assert_no_enqueued_jobs only: Huddle::PushInvitationJob do
      HuddleGrant.issue!(session: second_session_for(users(:david)), membership: @starter_membership)
    end

    assert_predicate item.reload, :handled?
    assert_equal 1, ActivityItem.where(user: users(:jason)).count
  end

  test "an invitation older than two minutes re-rings through the same row" do
    item = nil
    travel_to 3.minutes.ago do
      HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
      item = ActivityItem.find_by!(user: users(:jason), event_type: "huddle_started")
    end

    assert_enqueued_with(job: Huddle::PushInvitationJob, args: [ item.id ]) do
      HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
    end

    assert_equal 1, ActivityItem.where(user: users(:jason)).count
    assert_operator item.reload.created_at, :>, 1.minute.ago
  end

  test "a handled invitation older than two minutes re-rings through the same row" do
    item = nil
    travel_to 3.minutes.ago do
      HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
      item = ActivityItem.find_by!(user: users(:jason)).mark_handled!
    end

    assert_enqueued_with(job: Huddle::PushInvitationJob, args: [ item.id ]) do
      HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
    end

    assert_equal 1, ActivityItem.where(user: users(:jason)).count
    item.reload
    assert_equal "huddle_started", item.event_type
    assert_predicate item, :unread?
    assert_operator item.created_at, :>, 1.minute.ago
  end

  test "a missed invitation older than two minutes re-rings through the same row" do
    item = nil
    travel_to 3.minutes.ago do
      HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
      item = ActivityItem.find_by!(user: users(:jason))
      item.update!(event_type: "huddle_missed")
    end

    HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)

    item.reload
    assert_equal "huddle_started", item.event_type
    assert_predicate item, :unread?
    assert_equal 1, ActivityItem.where(user: users(:jason)).count
  end

  test "revoking the starter's in-call grant broadcasts call-ended to the invitee" do
    grant = HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
    item = ActivityItem.find_by!(user: users(:jason), source: grant)
    grant.update_columns(last_seen_at: Time.current)

    grant.revoke!

    assert_call_ended_broadcast(item)
    assert_equal "huddle_started", item.reload.event_type
  end

  test "the starter leaving the call broadcasts call-ended to the invitee" do
    grant = HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
    item = ActivityItem.find_by!(user: users(:jason), source: grant)
    grant.update_columns(last_seen_at: Time.current)

    assert grant.mark_out_of_call!

    assert_call_ended_broadcast(item)
  end

  test "revoking a quiet grant broadcasts no call-ended" do
    grant = HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)

    assert_broadcasts ActivityChannel.stream_name_for(users(:jason).id), 0 do
      grant.revoke!
    end
  end

  test "no call-ended when the recipient already joined" do
    david_grant = HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
    HuddleGrant.issue!(session: second_session_for(users(:jason)), membership: memberships(:jason_david_and_jason))
    david_grant.update_columns(last_seen_at: Time.current)

    assert_broadcasts ActivityChannel.stream_name_for(users(:jason).id), 0 do
      david_grant.revoke!
    end
  end

  test "a suppressed ring ends with a banner-only call-ended" do
    users(:jason).update!(inbox_preferences: { "huddle_invitations" => false })
    grant = HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
    grant.update_columns(last_seen_at: Time.current)

    grant.revoke!

    assert_broadcast_on(ActivityChannel.stream_name_for(users(:jason).id), {
      activityItemId: 0,
      huddleInvitation: {
        activityItemId: 0,
        eventType: "huddle_ended",
        state: "unread",
        roomId: @room.id,
        roomName: "David",
        roomPath: Rails.application.routes.url_helpers.room_path(@room),
        callerName: "David",
        readPath: "",
        handledPath: ""
      }
    })
  end

  test "no banner-only call-ended when the ring long stopped" do
    users(:jason).update!(inbox_preferences: { "huddle_invitations" => false })
    grant = HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)

    travel 5.minutes do
      grant.update_columns(last_seen_at: Time.current)

      assert_broadcasts ActivityChannel.stream_name_for(users(:jason).id), 0 do
        grant.revoke!
      end
    end
  end

  test "retrying after the window with a new grant reuses the unhandled item" do
    HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
    item = ActivityItem.find_by!(user: users(:jason))

    travel 46.seconds do
      Huddle::InvitationResolver.resolve_overdue!
    end
    assert_equal "huddle_missed", item.reload.event_type

    # The recipient never joined; the starter retries from another session
    # after the window with a brand-new grant. The same attempt re-rings
    # through its own row instead of stacking a second missed call beside
    # it, and resolving again still leaves exactly one missed item.
    travel 3.minutes do
      assert_enqueued_with(job: Huddle::PushInvitationJob, args: [ item.id ]) do
        HuddleGrant.issue!(session: second_session_for(users(:david)), membership: @starter_membership)
      end
    end

    assert_equal 1, ActivityItem.where(user: users(:jason)).count
    item.reload
    assert_equal "huddle_started", item.event_type
    assert_predicate item, :unread?

    travel 6.minutes do
      Huddle::InvitationResolver.resolve_overdue!
    end

    assert_equal "huddle_missed", item.reload.event_type
    assert_equal 1, ActivityItem.where(user: users(:jason)).count
  end

  test "a retry after a handled attempt opens a new item" do
    HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
    ActivityItem.find_by!(user: users(:jason)).mark_handled!

    # The recipient joined, closing the attempt; the next invite — even
    # from a new grant after the window — is a new attempt with its own
    # row.
    travel 3.minutes do
      HuddleGrant.issue!(session: second_session_for(users(:david)), membership: @starter_membership)
    end

    assert_equal 2, ActivityItem.where(user: users(:jason)).count
    assert_equal 1, ActivityItem.where(user: users(:jason), event_type: "huddle_started", handled_at: nil).count
  end

  test "a retry from the first device re-rings through its own item" do
    laptop = HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
    first_item = ActivityItem.find_by!(user: users(:jason), source: laptop)

    # The recipient joined, closing the attempt, then left again; the row
    # stays behind as handled history.
    HuddleGrant.issue!(session: second_session_for(users(:jason)), membership: memberships(:jason_david_and_jason))
    assert_predicate first_item.reload, :handled?

    travel 3.minutes do
      HuddleGrant.issue!(session: second_session_for(users(:david)), membership: @starter_membership)
    end
    second_item = ActivityItem.find_by!(user: users(:jason), event_type: "huddle_started", handled_at: nil)
    assert_equal 2, ActivityItem.where(user: users(:jason)).count

    # The laptop retries after the window. Repointing the phone's row at
    # the laptop grant would collide with the handled row the laptop
    # already owns, so the laptop re-rings through its own row instead —
    # and the recipient is rung rather than skipped.
    travel 6.minutes do
      assert_enqueued_with(job: Huddle::PushInvitationJob, args: [ first_item.id ]) do
        assert_equal laptop, HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
      end
    end

    assert_equal 2, ActivityItem.where(user: users(:jason)).count
    first_item.reload
    assert_equal "huddle_started", first_item.event_type
    assert_predicate first_item, :unread?
    assert_not_predicate first_item, :handled?
    assert_equal "huddle_started", second_item.reload.event_type
  end

  test "the same attempt reuses its item inside ten minutes and opens a new one after" do
    travel_to 20.minutes.ago do
      HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
    end
    item = ActivityItem.find_by!(user: users(:jason))

    Huddle::InvitationResolver.resolve_overdue!
    assert_equal "huddle_missed", item.reload.event_type

    # Nine minutes old: the same attempt, re-ringing through its own row.
    travel_to 11.minutes.ago do
      assert_enqueued_with(job: Huddle::PushInvitationJob, args: [ item.id ]) do
        HuddleGrant.issue!(session: second_session_for(users(:david)), membership: @starter_membership)
      end
    end
    assert_equal 1, ActivityItem.where(user: users(:jason)).count
    assert_equal "huddle_started", item.reload.event_type

    # Eleven minutes after that row was refreshed: a new attempt with its
    # own row, leaving the old one alone.
    assert_enqueued_with(job: Huddle::PushInvitationJob) do
      HuddleGrant.issue!(session: second_session_for(users(:david)), membership: @starter_membership)
    end

    assert_equal 2, ActivityItem.where(user: users(:jason)).count
    assert_equal "huddle_started", item.reload.event_type
    fresh = ActivityItem.where(user: users(:jason)).order(created_at: :desc).first
    assert_not_equal item.id, fresh.id
    assert_equal "huddle_started", fresh.event_type
    assert_not_predicate fresh, :handled?
  end

  test "obtaining a grant clears the recipient's open invitations for the room" do
    HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
    item = ActivityItem.find_by!(user: users(:jason), event_type: "huddle_started")

    HuddleGrant.issue!(session: second_session_for(users(:jason)), membership: memberships(:jason_david_and_jason))

    assert_predicate item.reload, :handled?
  end

  test "joining late clears the missed item" do
    HuddleGrant.issue!(session: @starter_session, membership: @starter_membership)
    item = ActivityItem.find_by!(user: users(:jason))

    travel 46.seconds do
      Huddle::InvitationResolver.resolve_overdue!
    end
    assert_equal "huddle_missed", item.reload.event_type

    HuddleGrant.issue!(session: second_session_for(users(:jason)), membership: memberships(:jason_david_and_jason))

    assert_predicate item.reload, :handled?
  end

  test "a DM with only bots besides the starter gets no invitation" do
    assert_no_difference -> { ActivityItem.count } do
      HuddleGrant.issue!(session: second_session_for(users(:kevin)), membership: memberships(:kevin_bender_and_kevin))
    end
  end

  test "issuing a grant in a group DM invites every other human member" do
    group_room = Rooms::Direct.create_for({ creator: users(:david) }, users: [ users(:david), users(:jason), users(:kevin) ])

    assert_enqueued_jobs 2, only: Huddle::PushInvitationJob do
      HuddleGrant.issue!(session: @starter_session, membership: group_room.memberships.find_by(user: users(:david)))
    end

    [ users(:jason), users(:kevin) ].each do |recipient|
      item = ActivityItem.find_by!(user: recipient, event_type: "huddle_started")
      assert_equal group_room.id, item.source.room_id
      assert_equal users(:david).id, item.source.user_id
      assert_predicate item, :unread?
    end
    assert_not ActivityItem.exists?(user: users(:david), event_type: "huddle_started")
  end

  test "group DM invitations skip bots and members who switched the room off" do
    group_room = Rooms::Direct.create_for({ creator: users(:david) },
      users: [ users(:david), users(:jason), users(:kevin), users(:bender) ])
    group_room.memberships.find_by!(user: users(:kevin)).update!(involvement: "nothing")

    HuddleGrant.issue!(session: @starter_session, membership: group_room.memberships.find_by(user: users(:david)))

    assert ActivityItem.exists?(user: users(:jason), event_type: "huddle_started")
    assert_not ActivityItem.exists?(user: users(:kevin), event_type: "huddle_started")
    assert_not ActivityItem.exists?(user: users(:bender), event_type: "huddle_started")
  end

  test "a removed group member gets no invitation and loses their grant" do
    group_room = Rooms::Direct.create_for({ creator: users(:david) },
      users: [ users(:david), users(:jason), users(:kevin), users(:jz) ])
    grant = HuddleGrant.issue!(session: second_session_for(users(:kevin)),
      membership: group_room.memberships.find_by(user: users(:kevin)))

    group_room.memberships.find_by!(user: users(:kevin)).destroy!
    assert_predicate grant.reload, :revoked?

    HuddleGrant.issue!(session: @starter_session, membership: group_room.memberships.find_by(user: users(:david)))

    assert ActivityItem.exists?(user: users(:jason), event_type: "huddle_started")
    assert ActivityItem.exists?(user: users(:jz), event_type: "huddle_started")
    assert_not ActivityItem.exists?(user: users(:kevin), event_type: "huddle_started")
  end

  private
    def second_session_for(user)
      Session.create!(user: user, user_agent: "second device", ip_address: "127.0.0.2")
    end

    def assert_call_ended_broadcast(item)
      routes = Rails.application.routes.url_helpers
      assert_broadcast_on(ActivityChannel.stream_name_for(item.user_id), {
        activityItemId: item.id,
        huddleInvitation: {
          activityItemId: item.id,
          eventType: "huddle_ended",
          state: "unread",
          roomId: @room.id,
          roomName: "David",
          roomPath: routes.room_path(@room),
          callerName: "David",
          readPath: routes.read_activity_item_path(item, state: "read"),
          handledPath: routes.handled_activity_item_path(item, state: "handled")
        }
      })
    end
end
