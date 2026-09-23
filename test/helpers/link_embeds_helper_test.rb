require "test_helper"

class LinkEmbedsHelperTest < ActionView::TestCase
  include LinkEmbedsHelper
  include Linkedin::PostsHelper

  test "link_embed_cards_for returns usable generic embeds in link order" do
    message = rooms(:designers).messages.create!(
      creator: users(:david), client_message_id: "embed-helper-order",
      markdown_source: "https://example.com/b https://example.com/a"
    )
    fresh_embed("https://example.com/a", title: "A")
    fresh_embed("https://example.com/b", title: "B")

    assert_equal %w[ B A ], link_embed_cards_for(message.reload).map(&:title)
  end

  test "generic and LinkedIn cards split by URL" do
    message = rooms(:designers).messages.create!(
      creator: users(:david), client_message_id: "embed-helper-split",
      markdown_source: "https://example.com/page https://www.linkedin.com/feed/update/urn:li:activity:7"
    )
    fresh_embed("https://example.com/page", title: "Page")
    fresh_embed("https://www.linkedin.com/feed/update/urn:li:activity:7", title: "Post")

    assert_equal [ "https://example.com/page" ], link_embed_cards_for(message.reload).map(&:normalized_url)
    assert_equal [ "https://www.linkedin.com/feed/update/urn:li:activity:7" ],
      linkedin_post_cards_for(message.reload).map(&:normalized_url)
  end

  test "unusable generic embeds are left out but LinkedIn falls back to a chip" do
    message = rooms(:designers).messages.create!(
      creator: users(:david), client_message_id: "embed-helper-unusable",
      markdown_source: "https://example.com/gated https://www.linkedin.com/posts/slug-8"
    )
    failed_embed("https://example.com/gated")
    failed_embed("https://www.linkedin.com/posts/slug-8")

    assert_empty link_embed_cards_for(message.reload)
    assert_equal 1, linkedin_post_cards_for(message.reload).size
  end

  test "suppressed messages render no cards" do
    message = rooms(:designers).messages.create!(
      creator: users(:david), client_message_id: "embed-helper-suppressed",
      markdown_source: "https://example.com/page"
    )
    fresh_embed("https://example.com/page", title: "Page")
    message.update!(embeds_suppressed: true)

    assert_empty link_embed_cards_for(message.reload)
    assert_empty linkedin_post_cards_for(message.reload)
  end

  test "stale cards re-enqueue their fetch once" do
    message = rooms(:designers).messages.create!(
      creator: users(:david), client_message_id: "embed-helper-reclaim",
      markdown_source: "https://example.com/stale"
    )
    embed = LinkEmbed.find_by!(normalized_url: "https://example.com/stale")
    embed.update_columns(expires_at: 1.hour.ago, fetch_requested_at: 1.hour.ago)
    clear_enqueued_jobs

    assert_enqueued_jobs 1, only: LinkEmbed::FetchJob do
      link_embed_cards_for(message.reload)
    end

    assert_no_enqueued_jobs only: LinkEmbed::FetchJob do
      link_embed_cards_for(message.reload)
    end
  end

  test "fresh cards enqueue nothing" do
    message = rooms(:designers).messages.create!(
      creator: users(:david), client_message_id: "embed-helper-fresh",
      markdown_source: "https://example.com/fresh"
    )
    fresh_embed("https://example.com/fresh", title: "Fresh")
    clear_enqueued_jobs

    assert_no_enqueued_jobs only: LinkEmbed::FetchJob do
      assert_equal 1, link_embed_cards_for(message.reload).size
    end
  end

  test "linkedin_embed_player_url follows the URN" do
    urn = LinkEmbed.new(url: "https://www.linkedin.com/feed/update/urn:li:share:9")
    slug = LinkEmbed.new(url: "https://www.linkedin.com/posts/slug-9")

    assert_equal "https://www.linkedin.com/embed/feed/update/urn:li:share:9", linkedin_embed_player_url(urn)
    assert_nil linkedin_embed_player_url(slug)
  end

  private
    def fresh_embed(normalized_url, title:)
      LinkEmbed.find_by!(normalized_url: normalized_url).update!(
        title: title, description: "#{title} description.", site_name: "Example",
        fetched_at: Time.current, fetch_error: nil, expires_at: 1.hour.from_now
      )
    end

    def failed_embed(normalized_url)
      LinkEmbed.find_by!(normalized_url: normalized_url).update!(
        fetched_at: Time.current, fetch_error: "No preview available for this link",
        expires_at: 1.hour.from_now
      )
    end
end
