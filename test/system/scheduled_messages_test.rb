require "application_system_test_case"

class ScheduledMessagesTest < ApplicationSystemTestCase
  setup do
    @room = rooms(:designers)
    sign_in "jz@37signals.com"
    join_room @room
  end

  test "schedules from the composer and lists in the Scheduled view" do
    fill_in_markdown "message_markdown_source", with: "Morning, team!"
    click_on "Schedule send"
    assert_selector "#schedule-send[open]", visible: true

    within "#schedule-send" do
      click_on "In 1 hour"
    end

    assert_selector "#composer .composer__feedback", text: "Scheduled for", visible: true
    assert_field "Write a message", with: ""

    visit scheduled_messages_url
    assert_selector "h1", text: "Scheduled"
    assert_selector ".scheduled-message__form textarea", text: "Morning, team!"
  end

  test "schedule send requires a draft" do
    click_on "Schedule send"

    within "#schedule-send" do
      click_on "Tomorrow at 9 AM"
      assert_selector ".schedule-send__status", text: "Write a message first."
    end
  end

  test "edits, sends now, and cancels from the Scheduled view" do
    editable = ScheduledMessage.create!(
      user: users(:jz), room: @room, markdown_source: "Draft one", send_at: 2.hours.from_now
    )
    sendable = ScheduledMessage.create!(
      user: users(:jz), room: @room, markdown_source: "Draft two", send_at: 2.hours.from_now
    )
    cancellable = ScheduledMessage.create!(
      user: users(:jz), room: @room, markdown_source: "Draft three", send_at: 2.hours.from_now
    )

    visit scheduled_messages_url

    within "##{ActionView::RecordIdentifier.dom_id(editable)}" do
      fill_in "Message", with: "Draft one, edited"
      click_on "Save"
    end
    assert_text "Scheduled message updated."
    assert_equal "Draft one, edited", editable.reload.markdown_source

    within "##{ActionView::RecordIdentifier.dom_id(sendable)}" do
      click_on "Send now"
    end
    assert_text "Message sent."
    assert sendable.reload.sent?

    visit room_url(@room)
    assert_message_text "Draft two", wait: 10

    visit scheduled_messages_url
    within "##{ActionView::RecordIdentifier.dom_id(cancellable)}" do
      click_on "Cancel"
    end
    assert_text "Scheduled message cancelled."
    assert_not ScheduledMessage.exists?(cancellable.id)
  end

  test "the sidebar links the Scheduled view" do
    visit scheduled_messages_url

    within ".sidebar" do
      click_on "Scheduled"
    end

    assert_selector "h1", text: "Scheduled"
  end
end
