require "test_helper"

class LinkEmbed::ReferenceSyncTest < ActiveSupport::TestCase
  setup do
    @room = rooms(:designers)
    @creator = users(:david)
  end

  test "syncs generic and LinkedIn references and enqueues one fetch per URL" do
    message = nil

    assert_enqueued_jobs 3, only: LinkEmbed::FetchJob do
      message = @room.messages.create!(
        creator: @creator, client_message_id: "embed-sync-basic",
        markdown_source: "read https://example.com/one and https://example.com/two " \
          "plus https://www.linkedin.com/feed/update/urn:li:activity:42"
      )
    end

    assert_equal %w[
      https://example.com/one https://example.com/two
      https://www.linkedin.com/feed/update/urn:li:activity:42
    ], message.reload.link_embeds.map(&:normalized_url).sort
    assert_equal [ 0, 1, 2 ], message.link_embed_references.order(:position).pluck(:position)
  end

  test "is idempotent and re-syncs on edit" do
    message = @room.messages.create!(
      creator: @creator, client_message_id: "embed-sync-edit",
      markdown_source: "read https://example.com/before"
    )
    clear_enqueued_jobs

    assert_no_enqueued_jobs only: LinkEmbed::FetchJob do
      LinkEmbed::ReferenceSync.call(message)
    end

    assert_enqueued_jobs 1, only: LinkEmbed::FetchJob do
      message.update!(markdown_source: "read https://example.com/after")
    end

    assert_equal [ "https://example.com/after" ], message.reload.link_embeds.map(&:normalized_url)
  end

  test "skips special URLs, code blocks, and angle-bracketed links" do
    message = @room.messages.create!(
      creator: @creator, client_message_id: "embed-sync-skips",
      markdown_source: <<~MD
        PR https://github.com/rails/rails/pull/1 and post https://x.com/jack/status/20
        <https://example.com/hidden> and <https://www.linkedin.com/posts/bracketed-1>
        `https://example.com/code`
        ```text
        https://example.com/fenced
        ```
        but https://example.com/kept stays
      MD
    )

    assert_equal [ "https://example.com/kept" ], message.reload.link_embeds.map(&:normalized_url)
  end

  test "a sentence-final LinkedIn link syncs one reference" do
    message = @room.messages.create!(
      creator: @creator, client_message_id: "embed-sync-linkedin-period",
      markdown_source: "see https://www.linkedin.com/posts/slug-55. Next sentence"
    )

    assert_equal [ "https://www.linkedin.com/posts/slug-55" ], message.reload.link_embeds.map(&:normalized_url)
  end

  test "syncs parenthesized URLs intact" do
    message = @room.messages.create!(
      creator: @creator, client_message_id: "embed-sync-parens",
      markdown_source: "see https://en.wikipedia.org/wiki/Rust_(programming_language) today"
    )

    assert_equal [ "https://en.wikipedia.org/wiki/Rust_(programming_language)" ],
      message.reload.link_embeds.map(&:normalized_url)
  end

  test "caps generic URLs at three but syncs LinkedIn URLs separately" do
    message = @room.messages.create!(
      creator: @creator, client_message_id: "embed-sync-caps",
      markdown_source: "https://example.com/1 https://example.com/2 https://example.com/3 " \
        "https://example.com/4 https://www.linkedin.com/posts/slug-99"
    )

    assert_equal %w[
      https://example.com/1 https://example.com/2 https://example.com/3
      https://www.linkedin.com/posts/slug-99
    ], message.reload.link_embeds.map(&:normalized_url).sort
  end

  test "legacy messages get no references" do
    message = @room.messages.create!(
      creator: @creator, client_message_id: "embed-sync-legacy",
      body: "<div>Look: https://example.com/legacy</div>"
    )

    assert_empty message.reload.link_embeds
  end

  test "suppressed messages keep references but enqueue no fetches" do
    message = @room.messages.create!(
      creator: @creator, client_message_id: "embed-sync-suppressed",
      markdown_source: "read https://example.com/suppressed"
    )
    message.link_embed_references.delete_all
    LinkEmbed.where(normalized_url: "https://example.com/suppressed").delete_all
    message.update_column(:embeds_suppressed, true)
    clear_enqueued_jobs

    assert_no_enqueued_jobs only: LinkEmbed::FetchJob do
      LinkEmbed::ReferenceSync.call(message)
    end

    assert_equal [ "https://example.com/suppressed" ], message.reload.link_embeds.map(&:normalized_url)
  end

  test "references keep each message's own raw URL while sharing one embed row" do
    first = @room.messages.create!(
      creator: @creator, client_message_id: "embed-sync-own-url-1",
      markdown_source: "diagram https://excalidraw.com/#json=AAA,111"
    )
    second = rooms(:hq).messages.create!(
      creator: @creator, client_message_id: "embed-sync-own-url-2",
      markdown_source: "diagram https://excalidraw.com/#json=BBB,222"
    )

    assert_equal first.link_embeds.first.id, second.link_embeds.first.id
    assert_equal "https://excalidraw.com/#json=AAA,111", first.link_embed_references.first.url
    assert_equal "https://excalidraw.com/#json=BBB,222", second.link_embed_references.first.url
  end

  test "editing a fragment updates the reference URL without a new fetch" do
    message = @room.messages.create!(
      creator: @creator, client_message_id: "embed-sync-fragment-edit",
      markdown_source: "diagram https://excalidraw.com/#json=AAA,111"
    )
    clear_enqueued_jobs

    assert_no_enqueued_jobs only: LinkEmbed::FetchJob do
      message.update!(markdown_source: "diagram https://excalidraw.com/#json=BBB,222")
    end

    assert_equal "https://excalidraw.com/#json=BBB,222", message.reload.link_embed_references.first.url
  end

  test "expired embeds refetch on the next sync" do
    message = @room.messages.create!(
      creator: @creator, client_message_id: "embed-sync-expired",
      markdown_source: "read https://example.com/stale"
    )
    embed = message.link_embeds.first
    # update_columns: a real update would broadcast, and the broadcast render
    # would already reclaim the expired embed before the sync runs.
    embed.update_columns(fetched_at: 2.days.ago, expires_at: 1.day.ago, fetch_requested_at: 2.days.ago)
    clear_enqueued_jobs

    assert_enqueued_jobs 1, only: LinkEmbed::FetchJob do
      LinkEmbed::ReferenceSync.call(message)
    end
  end
end
