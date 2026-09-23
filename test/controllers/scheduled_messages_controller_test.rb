require "test_helper"

# Simulates the dispatcher sending a row after the controller looked it
# up but before the controller takes the row lock.
module SendsBeforeLock
  def self.racing(scheduled)
    Thread.current[:sends_before_lock] = scheduled.id
    yield
  ensure
    Thread.current[:sends_before_lock] = nil
  end

  def lock!(*)
    if Thread.current[:sends_before_lock] == id
      ScheduledMessage.where(id: id).update_all(sent_at: Time.current)
    end
    super
  end
end
ScheduledMessage.prepend(SendsBeforeLock)

class ScheduledMessagesControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @user = users(:david)
    @room = rooms(:watercooler)
  end

  test "index lists upcoming and past rows" do
    upcoming = ScheduledMessage.create!(user: @user, room: @room, markdown_source: "Soon", send_at: 1.hour.from_now)
    sent = ScheduledMessage.create!(user: @user, room: @room, markdown_source: "Gone", send_at: 2.hours.from_now)
    sent.update_columns(sent_at: 1.hour.ago)

    get scheduled_messages_url

    assert_response :success
    assert_select "##{ActionView::RecordIdentifier.dom_id(upcoming)}"
    assert_select "##{ActionView::RecordIdentifier.dom_id(sent)}"
  end

  test "index hides other people's rows" do
    other = ScheduledMessage.create!(user: users(:jason), room: @room, markdown_source: "Theirs", send_at: 1.hour.from_now)

    get scheduled_messages_url

    assert_response :success
    assert_select "##{ActionView::RecordIdentifier.dom_id(other)}", count: 0
  end

  test "index shows stranded rows so they can be cancelled" do
    private_room = Rooms::Closed.create!(name: "Private", creator: users(:jason))
    private_room.memberships.grant_to users(:jason)
    private_room.memberships.grant_to @user
    stranded = ScheduledMessage.create!(user: @user, room: private_room, markdown_source: "Stranded", send_at: 1.hour.from_now)
    private_room.memberships.find_by(user: @user).destroy!

    get scheduled_messages_url

    assert_response :success
    assert_select "##{ActionView::RecordIdentifier.dom_id(stranded)}"

    delete scheduled_message_url(stranded)

    assert_redirected_to scheduled_messages_url
    assert_nil ScheduledMessage.find_by(id: stranded.id)
  end

  test "creates a scheduled message" do
    send_at = 1.hour.from_now

    assert_difference -> { @user.scheduled_messages.count }, 1 do
      post room_scheduled_messages_url(@room), params: {
        scheduled_message: { markdown_source: "Morning!", send_at: send_at.iso8601 }
      }, as: :json
    end

    assert_response :created
    assert_equal "Morning!", response.parsed_body["markdown_source"]
    assert_in_delta send_at.to_f, @user.scheduled_messages.ordered.last.send_at.to_f, 1
  end

  test "create rejects past times" do
    post room_scheduled_messages_url(@room), params: {
      scheduled_message: { markdown_source: "Late", send_at: 1.hour.ago.iso8601 }
    }, as: :json

    assert_response :unprocessable_entity
  end

  test "create is 404 outside membership" do
    private_room = Rooms::Closed.create!(name: "Private", creator: users(:jason))
    private_room.memberships.grant_to users(:jason)

    post room_scheduled_messages_url(private_room), params: {
      scheduled_message: { markdown_source: "Hi", send_at: 1.hour.from_now.iso8601 }
    }, as: :json

    assert_response :not_found
  end

  test "updates text and time" do
    scheduled = ScheduledMessage.create!(user: @user, room: @room, markdown_source: "Soon", send_at: 1.hour.from_now)

    patch scheduled_message_url(scheduled), params: {
      scheduled_message: { markdown_source: "Sooner!", send_at: 2.hours.from_now.strftime("%Y-%m-%dT%H:%M") }
    }, as: :json

    assert_response :success
    assert_equal "Sooner!", scheduled.reload.markdown_source
    assert_in_delta 2.hours.from_now.to_f, scheduled.send_at.to_f, 61
  end

  test "update during an active claim is refused" do
    scheduled = ScheduledMessage.create!(user: @user, room: @room, markdown_source: "Soon", send_at: 1.hour.from_now)
    scheduled.update_columns(claimed_at: Time.current)

    patch scheduled_message_url(scheduled), params: {
      scheduled_message: { markdown_source: "Edited!" }
    }, as: :json

    assert_response :conflict
    assert_match "sending right now", response.parsed_body["error"]
    assert_equal "Soon", scheduled.reload.markdown_source
  end

  test "update during an active claim redirects with a notice in HTML" do
    scheduled = ScheduledMessage.create!(user: @user, room: @room, markdown_source: "Soon", send_at: 1.hour.from_now)
    scheduled.update_columns(claimed_at: Time.current)

    patch scheduled_message_url(scheduled), params: {
      scheduled_message: { markdown_source: "Edited!" }
    }

    assert_redirected_to scheduled_messages_url
    assert_match "sending right now", flash[:alert]
    assert_equal "Soon", scheduled.reload.markdown_source
  end

  test "update after the claim goes stale is allowed" do
    scheduled = ScheduledMessage.create!(user: @user, room: @room, markdown_source: "Soon", send_at: 1.hour.from_now)
    scheduled.update_columns(claimed_at: 6.minutes.ago)

    patch scheduled_message_url(scheduled), params: {
      scheduled_message: { markdown_source: "Edited!" }
    }, as: :json

    assert_response :success
    assert_equal "Edited!", scheduled.reload.markdown_source
  end

  test "update is 404 for sent rows and other people's rows" do
    sent = ScheduledMessage.create!(user: @user, room: @room, markdown_source: "Gone", send_at: 2.hours.from_now)
    sent.update_columns(sent_at: 1.hour.ago)
    other = ScheduledMessage.create!(user: users(:jason), room: @room, markdown_source: "Theirs", send_at: 1.hour.from_now)

    patch scheduled_message_url(sent), params: { scheduled_message: { markdown_source: "Edit" } }, as: :json
    assert_response :not_found

    patch scheduled_message_url(other), params: { scheduled_message: { markdown_source: "Edit" } }, as: :json
    assert_response :not_found
  end

  test "destroy cancels pending rows only" do
    scheduled = ScheduledMessage.create!(user: @user, room: @room, markdown_source: "Soon", send_at: 1.hour.from_now)
    other = ScheduledMessage.create!(user: users(:jason), room: @room, markdown_source: "Theirs", send_at: 1.hour.from_now)

    assert_difference -> { ScheduledMessage.count }, -1 do
      delete scheduled_message_url(scheduled), as: :json
    end
    assert_response :no_content

    delete scheduled_message_url(other), as: :json
    assert_response :not_found
  end

  test "a send that lands between the lookup and the lock refuses the cancel" do
    scheduled = ScheduledMessage.create!(user: @user, room: @room, markdown_source: "Soon", send_at: 1.hour.from_now)

    SendsBeforeLock.racing(scheduled) do
      assert_no_difference -> { ScheduledMessage.count } do
        delete scheduled_message_url(scheduled), as: :json
      end
    end

    assert_response :conflict
  end

  test "a send that lands between the lookup and the lock refuses the edit" do
    scheduled = ScheduledMessage.create!(user: @user, room: @room, markdown_source: "Soon", send_at: 1.hour.from_now)

    SendsBeforeLock.racing(scheduled) do
      patch scheduled_message_url(scheduled), params: { scheduled_message: { markdown_source: "Edited!" } }, as: :json
    end

    assert_response :conflict
    assert_equal "Soon", scheduled.reload.markdown_source
  end

  test "destroy during an active claim is refused" do
    scheduled = ScheduledMessage.create!(user: @user, room: @room, markdown_source: "Soon", send_at: 1.hour.from_now)
    scheduled.update_columns(claimed_at: Time.current)

    assert_no_difference -> { ScheduledMessage.count } do
      delete scheduled_message_url(scheduled), as: :json
    end

    assert_response :conflict
    assert_match "sending right now", response.parsed_body["error"]
  end

  test "send_now posts immediately" do
    scheduled = ScheduledMessage.create!(user: @user, room: @room, markdown_source: "Now!", send_at: 2.hours.from_now)

    assert_difference -> { @room.messages.count }, 1 do
      post send_now_scheduled_message_url(scheduled), as: :json
    end

    assert_response :success
    assert scheduled.reload.sent?
  end

  test "send_now drops rows without access" do
    scheduled = ScheduledMessage.create!(user: @user, room: @room, markdown_source: "Stranded", send_at: 2.hours.from_now)
    @room.memberships.where(user: @user).delete_all

    assert_no_difference -> { Message.count } do
      post send_now_scheduled_message_url(scheduled), as: :json
    end

    assert_response :unprocessable_entity
    assert scheduled.reload.dropped?
  end

  test "send_now drops rows the model rejects with the reason" do
    board = Rooms::Board.create_for({ name: "Launch", creator: @user }, users: [ @user ])
    scheduled = ScheduledMessage.create!(user: @user, room: board, markdown_source: "Root post", send_at: 2.hours.from_now)

    assert_no_difference -> { Message.count } do
      post send_now_scheduled_message_url(scheduled), as: :json
    end

    assert_response :unprocessable_entity
    assert scheduled.reload.dropped?
    assert_match "board", response.parsed_body["error"]
  end

  test "bots are forbidden" do
    delete session_url
    bot = users(:bender)
    bot.update!(email_address: "bender@example.test", password: "secret123456")
    sign_in bot

    get scheduled_messages_url
    assert_response :forbidden
  end
end
