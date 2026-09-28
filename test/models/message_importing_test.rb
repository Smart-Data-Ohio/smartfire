require "test_helper"

# The Slack importer's quiet path: messages saved with `importing` keep
# rendering, mentions, search and DB-only references, and skip every noisy
# side effect. Normal saves behave exactly as before.
class MessageImportingTest < ActiveSupport::TestCase
  setup do
    @room = Rooms::Open.create!(name: "Import quiet", creator: users(:david))
    @author = users(:david)
    @member = users(:jason)
  end

  test "normal creation marks unread, pushes, records inbox items and fetches embeds" do
    assert_enqueued_with(job: Room::PushMessageJob) do
      assert_enqueued_with(job: LinkEmbed::FetchJob) do
        assert_difference("ActivityItem.count") do
          @room.messages.create!(creator: @author,
            markdown_source: "hi @[Jason] see https://example.com/article")
        end
      end
    end

    assert_not_nil @member.memberships.find_by(room: @room).unread_at
  end

  test "importing creation enqueues nothing and marks nothing unread" do
    assert_no_enqueued_jobs do
      assert_no_difference("ActivityItem.count") do
        @room.messages.create!(creator: @author, importing: true,
          markdown_source: "hi @[Jason] see https://example.com/article")
      end
    end

    assert_nil @member.memberships.find_by(room: @room).unread_at
    assert_nil @member.memberships.find_by(room: @room).last_read_message_id
  end

  test "importing creation keeps rendering, mentions, search and reference rows" do
    message = @room.messages.create!(creator: @author, importing: true,
      markdown_source: "hi @[Jason] see https://example.com/article")

    assert_includes message.body.body.to_html, "application/vnd.campfire.mention"
    assert_equal [ @member ], message.mentionees.to_a
    assert_includes Message.search("article"), message
    assert_equal 1, message.link_embed_references.count
  end

  test "normal thread replies refresh the count and importing replies wait for the importer" do
    parent = @room.messages.create!(creator: @author, markdown_source: "parent")
    thread = ChannelThread.create!(room: @room, creator: @author, parent_message: parent)

    @room.messages.create!(creator: @author, thread:, markdown_source: "normal reply")
    assert_equal 1, thread.reload.messages_count

    @room.messages.create!(creator: @author, thread:, importing: true, markdown_source: "quiet reply")
    assert_equal 1, thread.reload.messages_count

    ChannelThread.refresh_messages_count(thread.id)
    assert_equal 2, thread.reload.messages_count
  end

  test "destroying with importing removes the search row without touching quoters" do
    quoted = @room.messages.create!(creator: @author, importing: true, markdown_source: "quoted words")
    quoter = @room.messages.create!(creator: @author,
      markdown_source: "see /rooms/#{@room.id}/@#{quoted.id}")
    stamp = quoter.reload.updated_at

    quoted.importing = true
    quoted.destroy!

    assert_empty Message.search("quoted")
    assert_equal stamp, quoter.reload.updated_at
  end

  test "destroying normally still refreshes quoters" do
    quoted = @room.messages.create!(creator: @author, markdown_source: "quoted words")
    quoter = @room.messages.create!(creator: @author,
      markdown_source: "see /rooms/#{@room.id}/@#{quoted.id}")
    stamp = quoter.reload.updated_at

    travel 1.second do
      quoted.destroy!
    end

    assert_not_equal stamp, quoter.reload.updated_at
  end
end
