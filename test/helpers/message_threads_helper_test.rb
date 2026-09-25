require "test_helper"

class MessageThreadsHelperTest < ActionView::TestCase
  setup do
    @room = rooms(:designers)
    @parent = messages(:third)
  end

  test "a thread root with replies shows the count" do
    thread = start_thread
    post_reply(thread, "One")
    post_reply(thread, "Two")

    button = indicator_for(reloaded(@parent))

    assert_not button.key?("hidden")
    assert_equal "2 replies", button.at_css("span").text
    assert_equal "Open thread, 2 replies", button["aria-label"]
  end

  test "one reply is singular in the label and accessible name" do
    post_reply(start_thread, "Only")

    button = indicator_for(reloaded(@parent))

    assert_equal "1 reply", button.at_css("span").text
    assert_equal "Open thread, 1 reply", button["aria-label"]
  end

  test "a thread with no replies yet renders the target hidden, never 0 replies" do
    start_thread

    button = indicator_for(reloaded(@parent))

    assert button.key?("hidden")
    assert_equal "thread_indicator_message_#{@parent.client_message_id}", button["id"]
  end

  test "a message without a thread renders the target hidden" do
    assert indicator_for(reloaded(messages(:first))).key?("hidden")
  end

  test "system notes and streaming replies do not show in the count" do
    thread = start_thread
    post_reply(thread, "Real")
    Message.create!(room: @room, thread:, creator: users(:jz), system_note: true, markdown_source: "note")
    thread.post_message!(creator: users(:jz), attributes: { markdown_source: "Typing", streaming: true })

    assert_equal "1 reply", indicator_for(reloaded(@parent)).at_css("span").text
  end

  test "the icon follows the theme" do
    post_reply(start_thread, "One")

    icon = indicator_for(reloaded(@parent)).at_css("img")

    assert_includes icon["class"].split, "colorize--black"
    assert_equal "true", icon["aria-hidden"]
  end

  test "the pending-message template renders no indicator" do
    assert_empty render(partial: "messages/thread_indicator").strip
  end

  test "reading the count off a preloaded thread costs no query" do
    post_reply(start_thread, "One")
    message = reloaded(@parent)

    assert_no_queries { assert_equal 1, thread_reply_count(message) }
  end

  private
    def start_thread
      ChannelThread.create!(room: @room, creator: users(:jz), parent_message: @parent, name: "Indicator")
    end

    def post_reply(thread, text)
      thread.post_message!(creator: users(:jz), attributes: { markdown_source: text })
    end

    def reloaded(message)
      Message.with_rendering_details.find(message.id)
    end

    def indicator_for(message)
      Nokogiri::HTML5.fragment(render(partial: "messages/thread_indicator", locals: { message: })).at_css("button.message__thread-indicator")
    end
end
