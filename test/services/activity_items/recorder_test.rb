require "test_helper"

class ActivityItems::RecorderTest < ActiveSupport::TestCase
  setup do
    @room = rooms(:designers)
    @author = users(:jz)
    @recipient = users(:david)
  end

  test "records a mention for an opted-in active human and excludes the author" do
    message = @room.messages.create!(
      creator: @author,
      body: "Hey #{mention_attachment_for(:david)}",
      client_message_id: "activity-mention"
    )

    item = ActivityItem.find_by!(user: @recipient, source: message)
    assert_equal "mention", item.event_type
    assert_not ActivityItem.exists?(user: @author, source: message)
    assert_not ActivityItem.exists?(user: users(:bender), source: message)
  end

  test "a reply follows the reply author's current preference" do
    source = @room.messages.create!(creator: @recipient, body: "Original", client_message_id: "activity-reply-source")
    reply = @room.messages.create!(
      creator: @author,
      body: "Reply",
      reply_to_message: source,
      client_message_id: "activity-reply"
    )

    assert_equal "reply", ActivityItem.find_by!(user: @recipient, source: reply).event_type

    memberships(:david_designers).update!(involvement: "nothing")
    suppressed_source = @room.messages.create!(creator: @recipient, body: "Original 2", client_message_id: "activity-reply-source-2")
    suppressed_reply = @room.messages.create!(
      creator: @author,
      body: "Reply 2",
      reply_to_message: suppressed_source,
      client_message_id: "activity-reply-2"
    )
    assert_not ActivityItem.exists?(user: @recipient, source: suppressed_reply)
  end

  test "mention takes precedence when one message matches multiple activity reasons" do
    source = @room.messages.create!(creator: @recipient, body: "Original", client_message_id: "activity-priority-source")
    reply = @room.messages.create!(
      creator: @author,
      body: "Hey #{mention_attachment_for(:david)}",
      reply_to_message: source,
      client_message_id: "activity-priority"
    )

    assert_equal "mention", ActivityItem.find_by!(user: @recipient, source: reply).event_type
    assert_equal 1, ActivityItem.where(user: @recipient, source: reply).count
  end

  test "followed thread activity uses thread preferences" do
    thread = ChannelThread.create!(room: @room, creator: @author, name: "Activity thread")
    ThreadMembership.join!(thread, @author)
    ThreadMembership.join!(thread, @recipient).update!(involvement: "mentions")

    without_follow = thread.post_message!(creator: @author, attributes: { body: "Quiet update", client_message_id: "activity-thread-quiet" })
    assert_not ActivityItem.exists?(user: @recipient, source: without_follow)

    thread.memberships.find_by!(user: @recipient).update!(involvement: "everything")
    followed = thread.post_message!(creator: @author, attributes: { body: "Followed update", client_message_id: "activity-thread-followed" })
    assert_equal "thread_activity", ActivityItem.find_by!(user: @recipient, source: followed).event_type
  end

  test "work events notify followed thread members" do
    thread = ChannelThread.create!(room: @room, creator: @author, name: "Work activity thread")
    ThreadMembership.join!(thread, @author)
    ThreadMembership.join!(thread, @recipient).update!(involvement: "everything")

    thread.update_work!(actor: @author, work_status: "planned")

    event = thread.work_thread_events.ordered.first
    assert_equal "work_update", event.event_type
    assert_equal "work_update", ActivityItem.find_by!(user: @recipient, source: event).event_type
  end

  test "a direct mention reaches a member with notifications off but not an invisible one" do
    memberships(:david_designers).update!(involvement: "nothing")
    mention = @room.messages.create!(
      creator: @author,
      body: "Hey #{mention_attachment_for(:david)}",
      client_message_id: "activity-mention-nothing"
    )
    assert_equal "mention", ActivityItem.find_by!(user: @recipient, source: mention).event_type

    memberships(:david_designers).update!(involvement: "invisible")
    hidden = @room.messages.create!(
      creator: @author,
      body: "Hey #{mention_attachment_for(:david)}",
      client_message_id: "activity-mention-invisible"
    )
    assert_not ActivityItem.exists?(user: @recipient, source: hidden)
  end

  test "a member with notifications off gets no reply but a mentions member does" do
    memberships(:david_designers).update!(involvement: "nothing")
    source = @room.messages.create!(creator: @recipient, body: "Original", client_message_id: "activity-nothing-reply-source")
    suppressed = @room.messages.create!(
      creator: @author, body: "Reply", reply_to_message: source, client_message_id: "activity-nothing-reply"
    )
    assert_not ActivityItem.exists?(user: @recipient, source: suppressed)

    memberships(:david_designers).update!(involvement: "mentions")
    notified = @room.messages.create!(
      creator: @author, body: "Reply again", reply_to_message: source, client_message_id: "activity-mentions-reply"
    )
    assert_equal "reply", ActivityItem.find_by!(user: @recipient, source: notified).event_type
  end

  test "a member with notifications off gets no thread activity but a mentions member does" do
    thread = ChannelThread.create!(room: @room, creator: @author, name: "Involvement thread")
    ThreadMembership.join!(thread, @author)
    ThreadMembership.join!(thread, @recipient).update!(involvement: "everything")

    memberships(:david_designers).update!(involvement: "nothing")
    suppressed = thread.post_message!(creator: @author, attributes: { body: "Quiet", client_message_id: "activity-nothing-thread" })
    assert_not ActivityItem.exists?(user: @recipient, source: suppressed)

    memberships(:david_designers).update!(involvement: "mentions")
    notified = thread.post_message!(creator: @author, attributes: { body: "Loud", client_message_id: "activity-mentions-thread" })
    assert_equal "thread_activity", ActivityItem.find_by!(user: @recipient, source: notified).event_type
  end

  test "a member with notifications off gets no work items but a mentions member does" do
    thread = ChannelThread.create!(room: @room, creator: @author, name: "Involvement work thread")
    ThreadMembership.join!(thread, @author)
    ThreadMembership.join!(thread, @recipient).update!(involvement: "everything")

    memberships(:david_designers).update!(involvement: "nothing")
    thread.update_work!(actor: @author, work_status: "planned")
    thread.update_work!(actor: @author, work_owner_id: @recipient.id)
    assert_not ActivityItem.exists?(user: @recipient)

    memberships(:david_designers).update!(involvement: "mentions")
    thread.update_work!(actor: @author, work_status: "in_progress")
    assert_equal "work_update", ActivityItem.find_by!(user: @recipient).event_type
  end

  test "changing involvement leaves existing items untouched" do
    message = @room.messages.create!(
      creator: @author,
      body: "Hey #{mention_attachment_for(:david)}",
      client_message_id: "activity-untouched"
    )
    item = ActivityItem.find_by!(user: @recipient, source: message)

    memberships(:david_designers).update!(involvement: "nothing")
    assert_predicate item.reload, :unread?
    assert_equal "mention", item.event_type

    memberships(:david_designers).update!(involvement: "invisible")
    assert_predicate item.reload, :unread?
    assert_equal "mention", item.event_type
  end

  test "ten thread replies collapse into one thread activity item that reads unread again" do
    thread = ChannelThread.create!(room: @room, creator: @author, name: "Grouped thread")
    ThreadMembership.join!(thread, @author)
    ThreadMembership.join!(thread, @recipient).update!(involvement: "everything")

    previous = thread.post_message!(creator: @author, attributes: { body: "Update 0", client_message_id: "activity-grouped-0" })
    9.times do |index|
      previous = thread.post_message!(
        creator: @author,
        attributes: { body: "Update #{index + 1}", reply_to_message: previous, client_message_id: "activity-grouped-#{index + 1}" }
      )
    end

    items = ActivityItem.where(user: @recipient)
    assert_equal 1, items.count
    item = items.first
    assert_equal "thread_activity", item.event_type
    assert_predicate item, :unread?

    item.mark_read!
    assert_predicate item.reload, :read?

    thread.post_message!(
      creator: @author,
      attributes: { body: "Update 10", reply_to_message: previous, client_message_id: "activity-grouped-10" }
    )

    assert_equal 1, ActivityItem.where(user: @recipient).count
    assert_equal item.id, ActivityItem.find_by!(user: @recipient).id
    assert_predicate item.reload, :unread?
    assert_equal "Update 10", item.source.plain_text_body.strip
  end

  test "a status update after a work assignment keeps the assignment item and repoints the update item" do
    thread = ChannelThread.create!(room: @room, creator: @author, name: "Assigned thread")
    ThreadMembership.join!(thread, @author)
    ThreadMembership.join!(thread, @recipient).update!(involvement: "everything")

    thread.update_work!(actor: @author, work_status: "planned")
    thread.update_work!(actor: @author, work_owner_id: @recipient.id)
    thread.update_work!(actor: @author, work_status: "in_progress")

    items = ActivityItem.where(user: @recipient).order(:id)
    assert_equal %w[ work_update work_assignment ], items.pluck(:event_type)
    assert_equal "in_progress", items.first.source.to_status, "the collapsed update item points at the latest status change"
  end

  test "work updates for one thread collapse into a single item" do
    thread = ChannelThread.create!(room: @room, creator: @author, name: "Grouped work thread")
    ThreadMembership.join!(thread, @author)
    ThreadMembership.join!(thread, @recipient).update!(involvement: "everything")

    thread.update_work!(actor: @author, work_status: "planned")
    thread.update_work!(actor: @author, work_status: "in_progress")
    thread.update_work!(actor: @author, work_status: "blocked")

    items = ActivityItem.where(user: @recipient)
    assert_equal 1, items.count
    assert_equal "work_update", items.first.event_type
    assert_predicate items.first, :unread?
  end

  test "a thread message that mentions and replies to a follower yields one mention item" do
    thread = ChannelThread.create!(room: @room, creator: @author, name: "Precedence thread")
    ThreadMembership.join!(thread, @author)
    ThreadMembership.join!(thread, @recipient).update!(involvement: "everything")

    source = thread.post_message!(creator: @recipient, attributes: { body: "Original", client_message_id: "activity-thread-priority-source" })
    reply = thread.post_message!(
      creator: @author,
      attributes: {
        body: "Hey #{mention_attachment_for(:david)}",
        reply_to_message: source,
        client_message_id: "activity-thread-priority"
      }
    )

    assert_equal "mention", ActivityItem.find_by!(user: @recipient, source: reply).event_type
    assert_equal 1, ActivityItem.where(user: @recipient, source: reply).count
  end

  test "work assigned by an agent honors the recipient's agent_work switch" do
    thread = ChannelThread.create!(room: @room, creator: @author, name: "Agent work thread")
    ThreadMembership.join!(thread, @author)
    ThreadMembership.join!(thread, @recipient).update!(involvement: "everything")
    @recipient.update!(inbox_preferences: { "agent_work" => false })

    WorkThreadEvent.create!(
      thread:, actor: users(:bender), event_type: "work_assignment",
      from_status: "planned", to_status: "planned",
      to_owner_id: @recipient.id, to_owner_name: @recipient.name
    )
    assert_not ActivityItem.exists?(user: @recipient)

    WorkThreadEvent.create!(
      thread:, actor: users(:bender), event_type: "work_update",
      from_status: "planned", to_status: "in_progress"
    )
    assert_equal "work_update", ActivityItem.find_by!(user: @recipient).event_type
    ActivityItem.find_by!(user: @recipient).mark_handled!

    WorkThreadEvent.create!(
      thread:, actor: @author, event_type: "work_assignment",
      from_status: "planned", to_status: "planned",
      from_owner_id: @recipient.id, from_owner_name: @recipient.name,
      to_owner_id: users(:jason).id, to_owner_name: "Jason"
    )
    assert_equal "work_assignment", ActivityItem.find_by!(user: @recipient, handled_at: nil).event_type
  end

  test "work assigned by a bot without an agent ignores the agent_work switch" do
    thread = ChannelThread.create!(room: @room, creator: @author, name: "Bot work thread")
    ThreadMembership.join!(thread, @author)
    ThreadMembership.join!(thread, @recipient).update!(involvement: "everything")
    @recipient.update!(inbox_preferences: { "agent_work" => false })

    plain_bot = User.create_bot!(name: "Plain Bot #{SecureRandom.hex(4)}")
    assert_nil plain_bot.agent

    WorkThreadEvent.create!(
      thread:, actor: plain_bot, event_type: "work_assignment",
      from_status: "planned", to_status: "planned",
      to_owner_id: @recipient.id, to_owner_name: @recipient.name
    )
    assert_equal "work_assignment", ActivityItem.find_by!(user: @recipient).event_type
  end

  test "a caller-authorized record skips the source check but keeps idempotency" do
    thread = ChannelThread.create!(room: @room, creator: @author, name: "Board-like thread")
    ThreadMembership.join!(thread, @author)
    message = thread.post_message!(creator: @author,
      attributes: { body: "Opening", client_message_id: "activity-skip-check" })

    # David follows the room but never joined the thread, so the message
    # alone authorizes nothing for him.
    assert_nil ActivityItems::Recorder.record!(recipient: @recipient, source: message, event_type: "thread_activity")

    assert_difference -> { ActivityItem.where(user: @recipient, source: message).count }, 1 do
      2.times do
        ActivityItems::Recorder.record!(recipient: @recipient, source: message,
          event_type: "thread_activity", skip_source_check: true)
      end
    end
    assert_equal "thread_activity", ActivityItem.find_by!(user: @recipient, source: message).event_type
  end

  test "recording the same source twice is idempotent" do
    message = @room.messages.create!(
      creator: @author,
      body: "Hey #{mention_attachment_for(:david)}",
      client_message_id: "activity-idempotent"
    )
    ActivityItem.where(source: message).delete_all

    assert_difference -> { ActivityItem.where(user: @recipient, source: message).count }, 1 do
      2.times { ActivityItems::Recorder.record!(recipient: @recipient, source: message, event_type: "mention") }
    end
  end

  test "recording a thread message queries memberships a constant number of times as followers grow" do
    thread = ChannelThread.create!(room: @room, creator: @author, name: "Follower ceiling thread")
    ThreadMembership.join!(thread, @author)
    add_followers(thread, count: 5, offset: 0)
    message = thread.post_message!(creator: @author,
      attributes: { body: "Update", client_message_id: "activity-follower-ceiling" })

    ActivityItem.where(source: message).delete_all
    small = count_membership_queries { ActivityItems::Recorder.record_message!(message) }

    add_followers(thread, count: 25, offset: 5)
    ActivityItem.where(source: message).delete_all
    large = count_membership_queries { ActivityItems::Recorder.record_message!(message) }

    assert_equal small, large,
      "membership queries should stay constant, got #{small} for 5 followers and #{large} for 30"
  end

  test "a root message loads only the mentionee and reply-author memberships" do
    30.times { |i| @room.memberships.create!(user: User.create!(name: "Room member #{i}")) }
    source = @room.messages.create!(creator: @recipient, body: "Original",
      client_message_id: "activity-scoped-source")
    message = @room.messages.create!(
      creator: @author,
      body: "Hey #{mention_attachment_for(:david)}",
      reply_to_message: source,
      client_message_id: "activity-scoped-root"
    )

    memberships = ActivityItems::Recorder.new(message).send(:room_memberships)

    assert_equal [ users(:david).id ], memberships.keys
  end

  private
    def add_followers(thread, count:, offset:)
      count.times do |i|
        user = User.create!(name: "Follower #{offset + i}")
        @room.memberships.create!(user:)
        ThreadMembership.join!(thread, user).update!(involvement: "everything")
      end
    end

    # Membership lookups only: every recipient still needs its own activity
    # item row (plus its broadcast), so total queries grow with followers by
    # design and only the candidate computation must stay flat.
    def count_membership_queries
      count = 0
      subscription = ActiveSupport::Notifications.subscribe("sql.active_record") do |*, payload|
        count += 1 if payload[:name] != "SCHEMA" && !payload[:cached] && payload[:sql].match?(/membership/i)
      end

      ActiveRecord::Base.connection_pool.clear_query_cache
      yield
      count
    ensure
      ActiveSupport::Notifications.unsubscribe(subscription)
    end
end
