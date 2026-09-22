require "test_helper"

class Room::DestroyJobTest < ActiveJob::TestCase
  setup do
    @original_api_secret = ENV["LIVEKIT_API_SECRET"]
    ENV["LIVEKIT_API_SECRET"] = "test-api-secret"

    @david = users(:david)
    @jason = users(:jason)
    @room = Rooms::Stage.create_for({ name: "Town Hall", creator: @david }, users: [ @david, @jason, users(:kevin) ])

    @root_message = @room.root_messages.create!(creator: @david, markdown_source: "Hello")
    @thread = ChannelThread.create!(room: @room, creator: @david, name: "Planning")
    ThreadMembership.join!(@thread, @jason)
    @thread_message = @thread.post_message!(creator: @jason, attributes: { markdown_source: "Threaded" })
    @thread.update_work!(actor: @david, work_status: "planned")
    @work_event = @thread.work_thread_events.last!

    @pull_request = Github::PullRequest.create!(owner: "rails", repo: "rails", number: 12)
    @pr_thread = ChannelThread.create!(room: @room, creator: @david, name: "PR discussion")
    @pr_mapping = Github::PullRequestThread.create!(pull_request: @pull_request, room: @room, channel_thread: @pr_thread)
    Github::PullRequestReference.create!(message: @root_message, pull_request: @pull_request)

    @subscription = Github::RepositorySubscription.create!(
      room: @room, owner: "rails", repo: "rails", events: %w[opened], created_by: @david)
    @notification = Github::Notification.create!(subscription: @subscription, dedupe_key: "opened:rails/rails#12")

    @event = @room.events.create!(organizer: @david, title: "Planning session",
      starts_at: 2.days.from_now, time_zone: "UTC")
    @event.attendances.create!(user: @jason, response: "going")
    @calendar_entry = EventCalendarEntry.create!(event: @event, user: @david, google_event_id: "campfire-test-entry")
    EventReference.create!(message: @root_message, event: @event)

    membership = @room.memberships.find_by!(user: @david)
    @stream = Stream.create!(room: @room, membership:, user: @david, quality: "1080p15")

    @grant = HuddleGrant.issue!(session: sessions(:david_safari), membership:)
    @revoked_grant = HuddleGrant.issue!(session: @david.sessions.create!(user_agent: "Other"), membership:)
    @revoked_grant.revoke!

    agent = agents(:bender_agent)
    @agent_event = agent.agent_events.create!(event_type: "mention", room: @room,
      message: @root_message, outcome: "pending")
    @approval = AgentApproval.create!(agent:, room: @room, action: "github.comment",
      summary: "Comment on #12", expires_at: 1.hour.from_now)
    @agent_grant = AgentGrant.create!(agent:, capability: "read_messages", room: @room, granted_by: @david)

    Boost.create!(message: @root_message, booster: @jason, content: "Hello")
    @root_message.drive_attachments.create!(file_id: "1AbcDefGhIjKlMnOpQrSt")

    @message_item = ActivityItem.create!(user: @jason, source: @root_message, event_type: "mention")
    @work_item = ActivityItem.create!(user: @jason, source: @work_event, event_type: "work_update")
    @grant_item = ActivityItem.create!(user: @david, source: @grant, event_type: "huddle_missed")
    @event_item = ActivityItem.find_by!(user: @jason, source: @event)
  end

  teardown do
    ENV["LIVEKIT_API_SECRET"] = @original_api_secret
  end

  test "begin_destroy! immediately removes the room from everyone" do
    @room.begin_destroy!

    assert_predicate @room.reload, :deleted?
    assert_empty @room.memberships
    assert_predicate @grant.reload, :revoked?
    assert_predicate @agent_grant.reload, :revoked?
    assert_not_nil @stream.reload.ended_at
    # Content waits for the job.
    assert @room.messages.exists?
    assert Room.exists?(@room.id)
  end

  test "destroy removes the room and leaves no orphans" do
    room_id = @room.id
    event_id = @event.id
    thread_ids = @room.channel_thread_ids
    message_ids = @room.message_ids
    grant_ids = HuddleGrant.where(room_id:).ids
    source_item_ids = [ @message_item.id, @work_item.id, @grant_item.id, @event_item.id ]
    approval_item_ids = ActivityItem.where(source: @approval).ids
    assert_not_empty approval_item_ids

    @room.begin_destroy!
    Room::DestroyJob.perform_now(room_id)

    assert_empty Room.where(id: room_id)
    assert_empty Membership.where(room_id:)
    assert_empty Message.where(room_id:)
    assert_empty ChannelThread.where(room_id:)
    assert_empty Event.where(room_id:)
    assert_empty EventAttendance.where(event_id:)
    assert_empty EventCalendarEntry.where(event_id:)
    assert_empty Stream.where(room_id:)
    assert_empty HuddleGrant.where(room_id:)
    assert_empty ThreadMembership.where(thread_id: thread_ids)
    assert_empty Boost.where(message_id: message_ids)
    assert_empty DriveAttachment.where(message_id: message_ids)
    assert_empty Github::PullRequestReference.where(message_id: message_ids)
    assert_empty EventReference.where(message_id: message_ids)
    assert_empty Github::PullRequestThread.where(room_id:)
    assert_empty Github::RepositorySubscription.where(room_id:)
    assert_empty Github::Notification.where(id: @notification.id)
    assert_empty ActivityItem.where(id: source_item_ids)

    # The agent ledger outlives the room with its room link cleared.
    assert_empty AgentEvent.where(room_id:)
    assert_nil @agent_event.reload.room_id
    assert_empty AgentApproval.where(room_id:)
    assert_nil @approval.reload.room_id
    assert_equal approval_item_ids.sort, ActivityItem.where(source: @approval).ids.sort

    # Cleanup rows survive the grant unlink with their own room copies.
    assert HuddleCleanup.where(operation: :delete_room, room_name: @grant.room_name).exists?
    assert_empty HuddleCleanup.where(huddle_grant_id: grant_ids)
  end

  test "destroy keeps working when run twice" do
    @room.begin_destroy!
    Room::DestroyJob.perform_now(@room.id)
    Room::DestroyJob.perform_now(@room.id)

    assert_empty Room.where(id: @room.id)
  end

  test "destroy ignores rooms that were never marked" do
    Room::DestroyJob.perform_now(@room.id)

    assert Room.exists?(@room.id)
    assert_not_predicate @room.reload, :deleted?
  end

  test "destroy ignores missing rooms" do
    assert_no_difference -> { Room.count } do
      Room::DestroyJob.perform_now(Room.maximum(:id).to_i + 100)
    end
  end

  test "reenqueue_stuck! claims rooms so a second sweep enqueues nothing" do
    stuck = Rooms::Closed.create_for({ name: "Stuck", creator: @david }, users: [ @david ])
    stuck.begin_destroy!
    stuck.update_columns(deleted_at: 11.minutes.ago)

    assert_enqueued_with(job: Room::DestroyJob, args: [ stuck.id ]) do
      Room::DestroyJob.reenqueue_stuck!
    end
    assert_not_nil stuck.reload.destroy_enqueued_at

    assert_no_enqueued_jobs do
      Room::DestroyJob.reenqueue_stuck!
    end
  end

  test "reenqueue_stuck! re-enqueues once the claim expires" do
    stuck = Rooms::Closed.create_for({ name: "Stuck", creator: @david }, users: [ @david ])
    stuck.begin_destroy!
    stuck.update_columns(deleted_at: 11.minutes.ago)

    Room::DestroyJob.reenqueue_stuck!
    clear_enqueued_jobs

    travel_to 11.minutes.from_now do
      assert_enqueued_with(job: Room::DestroyJob, args: [ stuck.id ]) do
        Room::DestroyJob.reenqueue_stuck!
      end
    end
  end

  test "perform refreshes the sweep claim while the job runs" do
    @room.begin_destroy!
    @room.update_columns(deleted_at: 2.hours.ago, destroy_enqueued_at: 2.hours.ago)

    Room.any_instance.stubs(:destroy!).raises(Net::ReadTimeout, "boom")
    assert_enqueued_with(job: Room::DestroyJob, args: [ @room.id ]) do
      Room::DestroyJob.perform_now(@room.id)
    end

    assert_in_delta Time.current.to_i, @room.reload.destroy_enqueued_at.to_i, 5
  end

  test "destroy enqueue waits for the marking transaction to commit" do
    assert_no_enqueued_jobs do
      Room.transaction do
        @room.begin_destroy!
        Room::DestroyJob.perform_later(@room.id)
        raise ActiveRecord::Rollback
      end
    end

    assert_not_predicate @room.reload, :deleted?
  end

  test "destroy retries a transient failure and resumes where it stopped" do
    @room.begin_destroy!

    Room.any_instance.stubs(:destroy!).raises(Net::ReadTimeout, "boom")
    assert_enqueued_with(job: Room::DestroyJob, args: [ @room.id ]) do
      Room::DestroyJob.perform_now(@room.id)
    end

    # The content is already gone; the retry only has the room itself left.
    assert_empty @room.messages.reload
    Room.any_instance.unstub(:destroy!)

    assert_difference -> { Room.count }, -1 do
      Room::DestroyJob.perform_now(@room.id)
    end
  end

  test "destroy never loads more than a batch of messages at once" do
    room = Rooms::Closed.create_for({ name: "Batchy", creator: @david }, users: [ @david ])
    thread = ChannelThread.create!(room: room, creator: @david, name: "Big thread")
    5.times { |index| thread.post_message!(creator: @david, attributes: { markdown_source: "Message #{index}" }) }
    room.root_messages.create!(creator: @david, markdown_source: "Root")
    room.begin_destroy!

    counts = []
    with_destroy_batch_size(2) do
      ActiveSupport::Notifications.subscribed(
        ->(*, payload) { counts << payload[:record_count] if payload[:class_name] == "Message" },
        "instantiation.active_record"
      ) do
        Room::DestroyJob.perform_now(room.id)
      end
    end

    assert_not_empty counts
    assert_operator counts.max, :<=, 2
    assert_empty Message.where(room_id: room.id)
  end

  private
    def with_destroy_batch_size(size)
      original = Room::DestroyJob::BATCH_SIZE
      Room::DestroyJob.send(:remove_const, :BATCH_SIZE)
      Room::DestroyJob.const_set(:BATCH_SIZE, size)
      yield
    ensure
      Room::DestroyJob.send(:remove_const, :BATCH_SIZE)
      Room::DestroyJob.const_set(:BATCH_SIZE, original)
    end
end
