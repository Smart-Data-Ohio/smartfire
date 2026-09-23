require "test_helper"

class ActivityItems::RecorderKeywordTest < ActiveSupport::TestCase
  setup do
    @room = rooms(:designers)
    @author = users(:jz)
    @recipient = users(:david)
  end

  test "a keyword match on a room message records a keyword alert" do
    KeywordAlert.create!(user: @recipient, phrase: "deploy")

    message = @room.messages.create!(
      creator: @author, body: "Shipping the DEPLOY now", client_message_id: "keyword-room"
    )

    assert_equal "keyword_alert", ActivityItem.find_by!(user: @recipient, source: message).event_type
  end

  test "a keyword match needs a word boundary" do
    KeywordAlert.create!(user: @recipient, phrase: "deploy")

    message = @room.messages.create!(
      creator: @author, body: "Redeploying the service", client_message_id: "keyword-boundary"
    )

    assert_not ActivityItem.exists?(user: @recipient, source: message)
  end

  test "the author never matches their own keywords" do
    KeywordAlert.create!(user: @author, phrase: "deploy")

    message = @room.messages.create!(
      creator: @author, body: "Deploy now", client_message_id: "keyword-self"
    )

    assert_not ActivityItem.exists?(user: @author, source: message)
  end

  test "an invisible member matches nothing but a notifications-off member matches" do
    KeywordAlert.create!(user: @recipient, phrase: "deploy")

    memberships(:david_designers).update!(involvement: "invisible")
    hidden = @room.messages.create!(
      creator: @author, body: "Deploy now", client_message_id: "keyword-invisible"
    )
    assert_not ActivityItem.exists?(user: @recipient, source: hidden)

    memberships(:david_designers).update!(involvement: "nothing")
    notified = @room.messages.create!(
      creator: @author, body: "Deploy now", client_message_id: "keyword-nothing"
    )
    assert_equal "keyword_alert", ActivityItem.find_by!(user: @recipient, source: notified).event_type
  end

  test "a mention wins over a keyword match for the same message" do
    KeywordAlert.create!(user: @recipient, phrase: "deploy")

    message = @room.messages.create!(
      creator: @author,
      body: "Deploy now #{mention_attachment_for(:david)}",
      client_message_id: "keyword-precedence"
    )

    assert_equal "mention", ActivityItem.find_by!(user: @recipient, source: message).event_type
    assert_equal 1, ActivityItem.where(user: @recipient, source: message).count
  end

  test "a thread keyword match reaches thread members only" do
    KeywordAlert.create!(user: @recipient, phrase: "deploy")
    KeywordAlert.create!(user: users(:kevin), phrase: "deploy")

    thread = ChannelThread.create!(room: @room, creator: @author, name: "Keyword thread")
    ThreadMembership.join!(thread, @author)
    ThreadMembership.join!(thread, @recipient).update!(involvement: "mentions")

    message = thread.post_message!(creator: @author,
      attributes: { body: "Deploy now", client_message_id: "keyword-thread" })

    assert_equal "keyword_alert", ActivityItem.find_by!(user: @recipient, source: message).event_type
    assert_not ActivityItem.exists?(user: users(:kevin), source: message)
  end

  test "a muted thread member matches no keywords" do
    KeywordAlert.create!(user: @recipient, phrase: "deploy")

    thread = ChannelThread.create!(room: @room, creator: @author, name: "Muted keyword thread")
    ThreadMembership.join!(thread, @author)
    ThreadMembership.join!(thread, @recipient).update!(involvement: "nothing")

    message = thread.post_message!(creator: @author,
      attributes: { body: "Deploy now", client_message_id: "keyword-muted" })

    assert_not ActivityItem.exists?(user: @recipient, source: message)
  end

  test "thread activity wins over a keyword match for a follower" do
    KeywordAlert.create!(user: @recipient, phrase: "deploy")

    thread = ChannelThread.create!(room: @room, creator: @author, name: "Follower keyword thread")
    ThreadMembership.join!(thread, @author)
    ThreadMembership.join!(thread, @recipient).update!(involvement: "everything")

    message = thread.post_message!(creator: @author,
      attributes: { body: "Deploy now", client_message_id: "keyword-follower" })

    assert_equal "thread_activity", ActivityItem.find_by!(user: @recipient, source: message).event_type
    assert_equal 1, ActivityItem.where(user: @recipient, source: message).count
  end

  test "a room keyword match loads only the matching members' memberships" do
    KeywordAlert.create!(user: @recipient, phrase: "deploy")
    KeywordAlert.create!(user: users(:kevin), phrase: "deploy")
    3.times { |i| @room.memberships.create!(user: User.create!(name: "Room bystander #{i}")) }

    message = @room.messages.create!(
      creator: @author, body: "Deploy now", client_message_id: "keyword-scoped"
    )

    memberships = ActivityItems::Recorder.new(message).send(:room_memberships)

    assert_equal [ @recipient.id, users(:kevin).id ].sort, memberships.keys.sort
  end

  test "a room message keeps candidate queries flat as the roster grows" do
    KeywordAlert.create!(user: @recipient, phrase: "deploy")
    KeywordAlert.create!(user: users(:kevin), phrase: "deploy")
    add_room_members(count: 5, offset: 0)

    message = @room.messages.create!(
      creator: @author, body: "Deploy now", client_message_id: "keyword-room-ceiling"
    )

    ActivityItem.where(source: message).delete_all
    small = count_candidate_queries { ActivityItems::Recorder.record_message!(message) }

    add_room_members(count: 25, offset: 5)
    ActivityItem.where(source: message).delete_all
    large = count_candidate_queries { ActivityItems::Recorder.record_message!(message) }

    assert_equal small, large,
      "candidate queries should stay constant, got #{small} for a small roster and #{large} for a large one"
    assert_operator small, :>=, 1, "the probe message should actually exercise the keyword path"
  end

  test "matching queries the keyword table a constant number of times as followers grow" do
    thread = ChannelThread.create!(room: @room, creator: @author, name: "Keyword ceiling thread")
    ThreadMembership.join!(thread, @author)
    add_keyword_followers(thread, count: 5, offset: 0)

    message = thread.post_message!(creator: @author,
      attributes: { body: "Deploy now", client_message_id: "keyword-ceiling" })

    ActivityItem.where(source: message).delete_all
    small = count_keyword_queries { ActivityItems::Recorder.record_message!(message) }

    add_keyword_followers(thread, count: 25, offset: 5)
    ActivityItem.where(source: message).delete_all
    large = count_keyword_queries { ActivityItems::Recorder.record_message!(message) }

    assert_equal small, large,
      "keyword queries should stay constant, got #{small} for 5 followers and #{large} for 30"
    assert_operator small, :>=, 1, "the probe message should actually exercise the keyword path"
  end

  private
    def add_keyword_followers(thread, count:, offset:)
      count.times do |i|
        user = User.create!(name: "Keyword follower #{offset + i}")
        @room.memberships.create!(user:)
        ThreadMembership.join!(thread, user).update!(involvement: "mentions")
        KeywordAlert.create!(user:, phrase: "deploy")
      end
    end

    def add_room_members(count:, offset:)
      count.times do |i|
        @room.memberships.create!(user: User.create!(name: "Room member #{offset + i}"))
      end
    end

    def count_candidate_queries
      count = 0
      subscription = ActiveSupport::Notifications.subscribe("sql.active_record") do |*, payload|
        count += 1 if payload[:name] != "SCHEMA" && !payload[:cached] && payload[:sql].match?(/membership|keyword_alert/i)
      end

      ActiveRecord::Base.connection_pool.clear_query_cache
      yield
      count
    ensure
      ActiveSupport::Notifications.unsubscribe(subscription)
    end

    def count_keyword_queries
      count = 0
      subscription = ActiveSupport::Notifications.subscribe("sql.active_record") do |*, payload|
        count += 1 if payload[:name] != "SCHEMA" && !payload[:cached] && payload[:sql].match?(/keyword_alert/i)
      end

      ActiveRecord::Base.connection_pool.clear_query_cache
      yield
      count
    ensure
      ActiveSupport::Notifications.unsubscribe(subscription)
    end
end
