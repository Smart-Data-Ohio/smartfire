require "application_system_test_case"

class SendingMessagesTest < ApplicationSystemTestCase
  setup do
    sign_in "jz@37signals.com"
    join_room rooms(:designers)
  end

  test "sending messages between two users" do
    using_session("Kevin") do
      sign_in "kevin@37signals.com"
      join_room rooms(:designers)
    end

    join_room rooms(:designers)
    send_message "Is this thing on?"

    using_session("Kevin") do
      join_room rooms(:designers)
      assert_message_text "Is this thing on?"

      send_message "👍👍"
    end

    join_room rooms(:designers)
    assert_message_text "👍👍"
  end

  test "uploading a fresh video in the thread composer" do
    forgery_protection = ActionController::Base.allow_forgery_protection
    ActionController::Base.allow_forgery_protection = true
    room = rooms(:designers)
    thread = ChannelThread.create!(room: room, creator: users(:jz), name: "Video upload")
    ThreadMembership.join!(thread, users(:jz))
    visit room_url(room, thread: thread.id)
    wait_for_cable_connection

    within "#thread-panel" do
      assert_field "Write a thread reply", wait: BROADCAST_WAIT
      find("input[type=file]", visible: :all).set(file_fixture("alpha-centuri.mov").to_s)
      click_button "Send Reply"
      assert_selector "video.message__attachment", wait: BROADCAST_WAIT
    end

    perform_enqueued_jobs(only: Message::AttachmentProcessingJob)

    message = thread.messages.sole
    assert message.attachment.blob.service.exist?(message.attachment.key)
    preview = message.attachment.preview(format: :webp)
    assert preview.image.blob.service.exist?(preview.image.key)
    variant = preview.image.blob.variant_records.find_by!(variation_digest: preview.variation.digest)
    assert variant.image.blob.service.exist?(variant.image.key)
    assert_selector "#thread-panel div[style*='aspect-ratio'] video.message__attachment[poster]", wait: BROADCAST_WAIT
  ensure
    ActionController::Base.allow_forgery_protection = forgery_protection
  end

  test "editing messages" do
    using_session("Kevin") do
      sign_in "kevin@37signals.com"
      join_room rooms(:designers)
    end

    within_message messages(:third) do
      right_click_message
    end
    assert_message_menu_open
    click_on "Edit message", exact: true
    assert_selector "#composer", text: "Editing Message"
    fill_in_markdown "Write a message", with: "Redacted!"
    click_on "Send Message"
    assert_message_text "Redacted!"

    using_session("Kevin") do
      join_room rooms(:designers)

      assert_message_text "Redacted!"
    end
  end

  test "deleting messages" do
    using_session("Kevin") do
      sign_in "kevin@37signals.com"
      join_room rooms(:designers)

      assert_message_text "Third time's a charm."
    end

    within_message messages(:third) do
      right_click_message
    end
    assert_message_menu_open
    accept_confirm do
      click_on "Delete message"
    end

    using_session("Kevin") do
      assert_message_text "Third time's a charm.", count: 0
    end
  end
end
