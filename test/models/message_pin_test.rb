require "test_helper"

class MessagePinTest < ActiveSupport::TestCase
  include ActionCable::TestHelper

  setup do
    @room = rooms(:designers)
    @message = messages(:first)
    @pinner = users(:david)
  end

  test "pinning records the pin and posts a channel note as the pinner" do
    assert_difference -> { @room.message_pins.count }, 1 do
      assert_difference -> { @room.messages.count }, 1 do
        MessagePin.pin!(message: @message, pinner: @pinner)
      end
    end

    pin = MessagePin.last
    assert_equal @message, pin.message
    assert_equal @room, pin.room
    assert_equal @pinner, pin.pinner
    assert MessagePin.pinned?(@message)

    note = @room.messages.ordered.last
    assert_equal @pinner, note.creator
    assert_includes note.plain_text_body, "pinned a message"
    link = Nokogiri::HTML5.fragment(note.body.body.to_html).at_css("a")
    assert_equal "jump to message", link.text
    assert_equal Rails.application.routes.url_helpers.room_at_message_path(@room, @message), link["href"]
  end

  test "pinning broadcasts the badge, count, and panel list" do
    assert_turbo_stream_broadcasts [ @room, :messages ], count: 4 do
      MessagePin.pin!(message: @message, pinner: @pinner)
    end
  end

  test "pin notes are quiet system notes: visible, but no unread, push, delivery, inbox, or search" do
    member_stream = UnreadRoomsChannel.stream_name_for(users(:jason).id)

    assert_no_enqueued_jobs do
      assert_no_broadcasts member_stream do
        assert_no_difference -> { ActivityItem.count } do
          assert_no_changes -> { memberships(:jason_designers).reload.unread_at } do
            MessagePin.pin!(message: @message, pinner: @pinner)
          end
        end
      end
    end

    note = @room.messages.ordered.last
    assert_predicate note, :system_note?
    assert_not_includes @room.messages.search("pinned"), note
  end

  test "pin notes by agents record no delivery events" do
    assert_no_difference -> { AgentEvent.count } do
      assert_no_enqueued_jobs do
        MessagePin.pin!(message: @message, pinner: users(:bender))
      end
    end
  end

  test "repeated pin toggles post at most one note per message per 10 minutes" do
    assert_difference -> { @room.messages.count }, 1 do
      5.times do
        MessagePin.pin!(message: @message, pinner: @pinner).unpin!
      end
    end

    travel_to 11.minutes.from_now do
      assert_difference -> { @room.messages.count }, 1 do
        MessagePin.pin!(message: @message, pinner: @pinner)
      end
    end
  end

  test "pinning the same message twice is invalid" do
    MessagePin.pin!(message: @message, pinner: @pinner)

    assert_raises ActiveRecord::RecordInvalid do
      MessagePin.pin!(message: @message, pinner: users(:jason))
    end
  end

  test "pins cap at 50 per room" do
    MessagePin::MAX_PER_ROOM.times do |n|
      message = @room.root_messages.create!(creator: @pinner, markdown_source: "Pinnable #{n}", client_message_id: "cap-#{n}")
      MessagePin.pin!(message:, pinner: @pinner)
    end

    extra = @room.root_messages.create!(creator: @pinner, markdown_source: "One too many", client_message_id: "cap-extra")

    error = assert_raises MessagePin::CapReachedError do
      MessagePin.pin!(message: extra, pinner: @pinner)
    end
    assert_equal "This channel already has 50 pinned messages", error.message
    assert_equal MessagePin::MAX_PER_ROOM, @room.message_pins.count
  end

  test "unpinning removes the pin and broadcasts without a note" do
    pin = MessagePin.pin!(message: @message, pinner: @pinner)
    ActionCable.server.pubsub.clear

    assert_turbo_stream_broadcasts [ @room, :messages ], count: 3 do
      assert_no_difference -> { @room.messages.count } do
        assert_difference -> { @room.message_pins.count }, -1 do
          pin.unpin!
        end
      end
    end

    assert_not MessagePin.pinned?(@message)
  end

  test "pins order newest first" do
    first = MessagePin.pin!(message: messages(:first), pinner: @pinner)
    second = MessagePin.pin!(message: messages(:second), pinner: @pinner)

    assert_equal [ second, first ], @room.message_pins.ordered.to_a
  end

  test "destroying the message destroys its pin" do
    MessagePin.pin!(message: @message, pinner: @pinner)

    assert_difference -> { MessagePin.count }, -1 do
      @message.destroy!
    end
  end
end
