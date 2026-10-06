require "application_system_test_case"

class SearchForwardEditTest < ApplicationSystemTestCase
  setup do
    @room = rooms(:designers)
    sign_in "jz@37signals.com"
    join_room @room
  end

  test "search tolerates operators, shows an empty state and pages older results" do
    @room.messages.create!(
      creator: users(:jz), markdown_source: "system paging alpha",
      client_message_id: "system-search-alpha", created_at: 1.hour.ago
    )
    41.times do |index|
      @room.messages.create!(
        creator: users(:jz), markdown_source: "system paging filler #{index}",
        client_message_id: "system-search-filler-#{index}"
      )
    end

    visit room_url(@room)
    find("#global-search-input").click
    find("#global-search-input").send_keys "nonsense zebra tuxedo xyzzy", :enter
    assert_text "No messages match", wait: 10
    assert_field "global-search-input", with: "nonsense zebra tuxedo xyzzy"

    visit searches_url(q: "NOT")
    assert_selector "#message-area", wait: 10

    visit searches_url(q: "system paging")
    assert_selector "#search-results .message", count: 40, wait: 10
    assert_no_text "system paging alpha"
    click_link "Load older results"
    assert_text "system paging alpha", wait: 10
    assert_selector "#search-results .message", count: 42
  end

  test "forwarded Markdown keeps tables and code blocks" do
    source = @room.messages.create!(
      creator: users(:jz),
      markdown_source: "| Keep |\n| --- |\n| row |\n\n```ruby\nputs :forwarded\n```",
      client_message_id: "system-forward-source"
    )
    visit room_url(@room)
    assert_text "puts :forwarded", wait: 10

    open_message_menu(source)
    click_button "Forward"
    assert_selector "dialog[open]", visible: true, wait: 10
    find(".message-forward-dialog__destination:not(.message-forward-dialog__destination--thread)", text: "Designers", wait: 10).click
    within "dialog[open]" do
      click_button "Forward"
    end
    assert_selector "[data-message-actions-target='forwardStatus']", text: /Forwarded to 1 destination/, wait: 10

    forwarded = Message.where(forwarded_from_message: source).order(:id).last
    within "##{dom_id(forwarded)}" do
      assert_selector ".markdown-body table", wait: 10
      assert_selector ".markdown-body pre code.language-ruby", text: "puts :forwarded"
    end
  end

  test "editing to add a URL renders its card live and the edited marker on load" do
    message = @room.messages.create!(
      creator: users(:jz), markdown_source: "nothing linked yet", client_message_id: "system-edit-card"
    )
    visit room_url(@room)
    assert_text "nothing linked yet", wait: 10

    open_message_menu(message)
    click_button "Edit message"
    assert_selector "[data-composer-target='contextLabel']", text: "Editing Message", wait: 10
    fill_in "Write a message", with: "now with https://x.com/jack/status/424242"
    click_button "Send Message"

    within "##{dom_id(message)}" do
      assert_selector ".x-post-card", text: /Loading post/, wait: ApplicationSystemTestCase::BROADCAST_WAIT
    end

    visit room_url(@room)
    within "##{dom_id(message)}" do
      assert_selector ".x-post-card", wait: 10
      assert_selector ".message__edited", text: "(edited)"
    end
  end
end
