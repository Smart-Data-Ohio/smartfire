require "application_system_test_case"

class PollsTest < ApplicationSystemTestCase
  setup do
    @room = rooms(:designers)
    sign_in "jz@37signals.com"
    join_room @room
  end

  test "creates a poll from the builder and changes a vote" do
    fill_in_markdown "message_markdown_source", with: "/poll"
    click_on "Send Message"
    assert_selector "#poll-builder[open]", visible: true

    within "#poll-builder" do
      fill_in "Question", with: "Lunch?"
      fill_in "Option 1", with: "Tacos"
      fill_in "Option 2", with: "Pizza"
      click_on "Add option"
      fill_in "Option 3 (optional)", with: "Sushi"
      click_on "Post poll"
    end

    assert_message_text "Lunch?", wait: 10
    poll = Poll.order(:id).last
    card = "##{ActionView::RecordIdentifier.dom_id(poll, :card)}"
    assert_selector "#{card} .poll__label", text: "Sushi"

    # Each vote replaces the card over the stream, so every step
    # re-scopes to the fresh node.
    within card do
      choose "Tacos"
      click_on "Vote"
    end
    assert_selector "#{card} .poll__option--voted .poll__label", text: "Tacos"
    assert_selector "#{card} .poll__count", text: "1 · 100%"
    assert_selector "#{card} .poll__voters", text: "JZ"

    within card do
      choose "Pizza"
      click_on "Change vote"
    end
    assert_selector "#{card} .poll__option--voted .poll__label", text: "Pizza"

    within card do
      click_on "Retract vote"
    end
    assert_no_selector "#{card} .poll__option--voted"
    assert_selector "#{card} .poll__meta", text: "0 votes"
  end

  test "multiple-choice anonymous polls hide voters" do
    message = @room.root_messages.create!(creator: users(:jz), markdown_source: "Snacks?")
    poll = Poll.create_for_message!(message: message, labels: [ "Chips", "Fruit" ], multiple: true, anonymous: true)
    visit room_url(@room)
    wait_for_cable_connection

    card = "##{ActionView::RecordIdentifier.dom_id(poll, :card)}"
    within card do
      check "Chips"
      check "Fruit"
      click_on "Vote"

      assert_text "2 votes"
      assert_no_text "JZ"
    end

    assert_selector "#{card} .poll__meta", text: "Anonymous"
  end

  test "results update live in another session" do
    message = @room.root_messages.create!(creator: users(:jz), markdown_source: "Lunch?")
    poll = create_poll(message)
    card = "##{ActionView::RecordIdentifier.dom_id(poll, :card)}"

    using_session("Voter") do
      sign_in "jason@37signals.com"
      visit room_path(@room)
      wait_for_cable_connection
      assert_selector card
    end

    within card do
      choose "Tacos"
      click_on "Vote"
      assert_selector ".poll__count", text: "1 · 100%"
    end

    using_session("Voter") do
      assert_selector "#{card} .poll__count", text: "1 · 100%", wait: BROADCAST_WAIT
      assert_selector "#{card} .poll__voters", text: "JZ"
    end
  end

  test "closed polls show results without the form" do
    message = @room.root_messages.create!(creator: users(:jz), markdown_source: "Lunch?")
    poll = create_poll(message, closes_at: 1.minute.from_now)
    poll.update_columns(closes_at: 1.minute.ago)

    visit room_url(@room)
    wait_for_cable_connection

    card = "##{ActionView::RecordIdentifier.dom_id(poll, :card)}"
    assert_selector "#{card} .poll__meta", text: "Closed"
    assert_no_selector "#{card} form"
  end

  private
    def create_poll(message, **options)
      Poll.create_for_message!(message: message, labels: [ "Tacos", "Pizza" ], **options)
    end
end
