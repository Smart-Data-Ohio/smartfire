require "test_helper"

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

  test "bots are forbidden" do
    delete session_url
    bot = users(:bender)
    bot.update!(email_address: "bender@example.test", password: "secret123456")
    sign_in bot

    get scheduled_messages_url
    assert_response :forbidden
  end
end
