require "test_helper"

class Message::MentionPreloaderTest < ActiveSupport::TestCase
  setup do
    @room = rooms(:designers)
    Current.mentioned_users_by_id = nil
  end

  teardown do
    Current.mentioned_users_by_id = nil
  end

  test "preload_for resolves mentions from memory without further queries" do
    # Fixture access reads through the database on first touch, so resolve
    # it before counting.
    jason = users(:jason)
    message = create_mention_message("preload-resolve", "hey @[Jason]")
    loaded = Message.with_rendering_details.find(message.id)

    # The mentioned users plus their avatar attachments: the body scan
    # itself must not resolve anything.
    assert_queries_count 2 do
      Message::MentionPreloader.preload_for([ loaded ])
    end

    assert_no_queries do
      assert_equal jason, loaded.body.body.attachables.first
      assert_includes loaded.plain_text_body, "@Jason"
    end
  end

  test "mentions still resolve through the normal lookup when nothing is preloaded" do
    message = create_mention_message("preload-fallback", "hey @[Jason]")

    assert_equal users(:jason), message.body.body.attachables.first
  end

  test "garbage sgids resolve to no user instead of raising" do
    room = rooms(:pets).tap { |record| record.extend(ActionText::Attachable) }

    assert_nil Message::MentionPreloader.user_id_for_sgid("not-a-sgid")
    assert_nil Message::MentionPreloader.user_id_for_sgid(nil)
    assert_nil Message::MentionPreloader.user_id_for_sgid(room.attachable_sgid)
  end

  private
    def create_mention_message(client_message_id, source)
      @room.messages.create!(creator: users(:david),
        markdown_source: source, client_message_id:)
    end
end
