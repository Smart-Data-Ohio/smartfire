require "application_system_test_case"

class LinkEmbedsTest < ApplicationSystemTestCase
  setup do
    @room = rooms(:designers)
    sign_in "jz@37signals.com"
  end

  test "generic embeds render beneath the message" do
    message = @room.messages.create!(
      creator: users(:jz), client_message_id: "sys-embed-generic",
      markdown_source: "read https://example.com/sys-article today"
    )
    fresh_embed("https://example.com/sys-article",
      title: "A System Article", description: "Rendered in the browser.",
      site_name: "Example News", image_url: nil)

    join_room @room

    within "##{dom_id(message, :link_embed_cards)}" do
      assert_selector ".link-embed-card__site", text: "Example News"
      assert_selector ".link-embed-card__title", text: "A System Article"
      assert_selector ".link-embed-card__description", text: "Rendered in the browser."
    end
  end

  test "LinkedIn cards offer a click-to-load embedded post" do
    message = @room.messages.create!(
      creator: users(:jz), client_message_id: "sys-embed-linkedin",
      markdown_source: "see https://www.linkedin.com/feed/update/urn:li:activity:600600600"
    )
    fresh_embed("https://www.linkedin.com/feed/update/urn:li:activity:600600600",
      title: "JZ on shipping", description: "We shipped it.", site_name: "LinkedIn", image_url: nil)

    join_room @room

    within "##{dom_id(message, :linkedin_cards)}" do
      assert_selector ".linkedin-post-card__source", text: "LinkedIn"
      assert_selector ".linkedin-post-card__title", text: "JZ on shipping"
      assert_no_selector "iframe"

      click_button "Show embedded post"
      assert_selector "iframe.linkedin-post-card__player[src=?]",
        "https://www.linkedin.com/embed/feed/update/urn:li:activity:600600600"
      assert_no_button "Show embedded post"
    end
  end

  test "login-gated LinkedIn pages render a compact chip" do
    message = @room.messages.create!(
      creator: users(:jz), client_message_id: "sys-embed-chip",
      markdown_source: "see https://www.linkedin.com/posts/sys-gated-700"
    )
    LinkEmbed.find_by!(normalized_url: "https://www.linkedin.com/posts/sys-gated-700").update!(
      fetched_at: Time.current, fetch_error: "No preview available for this link",
      expires_at: 1.hour.from_now
    )

    join_room @room

    within "##{dom_id(message, :linkedin_cards)}" do
      assert_selector ".linkedin-post-chip a", text: "View post on LinkedIn"
      assert_no_selector ".linkedin-post-card"
    end
  end

  test "the author removes embeds from the message menu" do
    message = @room.messages.create!(
      creator: users(:jz), client_message_id: "sys-embed-remove",
      markdown_source: "read https://example.com/sys-remove"
    )
    fresh_embed("https://example.com/sys-remove",
      title: "Removable", description: "Going away.", site_name: "Example", image_url: nil)

    join_room @room
    assert_selector "##{dom_id(message, :link_embed_cards)} .link-embed-card", wait: 10

    within_message(message) do
      right_click_message
    end
    assert_message_menu_open
    click_button "Remove embeds"

    assert_no_selector "##{dom_id(message, :link_embed_cards)} .link-embed-card", wait: 10
    assert_predicate message.reload, :embeds_suppressed?
  end

  test "other members see no remove-embeds action" do
    message = @room.messages.create!(
      creator: users(:david), client_message_id: "sys-embed-noaction",
      markdown_source: "read https://example.com/sys-other"
    )
    fresh_embed("https://example.com/sys-other",
      title: "Not mine", description: "Stays.", site_name: "Example", image_url: nil)

    join_room @room
    assert_selector "##{dom_id(message, :link_embed_cards)} .link-embed-card", wait: 10

    within_message(message) do
      right_click_message
    end
    assert_message_menu_open
    # Quick reactions gain aria-pressed only once the metadata lands; without
    # this settle signal the absence below could pass before it arrives.
    assert_selector "#message-actions-menu [data-reaction][aria-pressed]", wait: 10
    assert_no_button "Remove embeds", visible: true
  end

  private
    def fresh_embed(normalized_url, title:, description:, site_name:, image_url:)
      LinkEmbed.find_by!(normalized_url: normalized_url).update!(
        title: title, description: description, site_name: site_name, image_url: image_url,
        fetched_at: Time.current, fetch_error: nil, expires_at: 1.hour.from_now
      )
    end

    def dom_id(record, prefix)
      ActionView::RecordIdentifier.dom_id(record, prefix)
    end
end
