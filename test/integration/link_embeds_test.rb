require "test_helper"

class LinkEmbedsTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @room = rooms(:designers)
  end

  test "room view renders generic embeds with site, title, description, and image" do
    stub_page("https://example.com/article", <<~HTML)
      <html><head>
        <meta property="og:title" content="A Great Article">
        <meta property="og:description" content="Worth the read.">
        <meta property="og:site_name" content="Example News">
        <meta property="og:image" content="https://example.com/hero.png">
      </head></html>
    HTML
    WebMock.stub_request(:head, "https://example.com/hero.png")
      .to_return(status: 200, headers: { content_type: "image/png" })

    message = @room.messages.create!(
      creator: users(:jason), client_message_id: "embed-room-generic",
      markdown_source: "read https://example.com/article today"
    )
    perform_enqueued_jobs only: LinkEmbed::FetchJob

    get room_url(@room)

    assert_response :success
    assert_select "##{dom_id(message, :link_embed_cards)}" do
      assert_select ".link-embed-card", count: 1
      assert_select ".link-embed-card__site", text: "Example News"
      assert_select ".link-embed-card__title a[href=?]", "https://example.com/article", text: "A Great Article"
      assert_select ".link-embed-card__description", text: "Worth the read."
      assert_select "img.link-embed-card__image[src=?]", "https://example.com/hero.png"
    end
  end

  test "card text is escaped exactly once" do
    message = @room.messages.create!(
      creator: users(:jason), client_message_id: "embed-escape-once",
      markdown_source: "read https://example.com/escape"
    )
    LinkEmbed.find_by!(normalized_url: "https://example.com/escape").update!(
      title: 'Tom & Jerry say "hi" <3', description: "5 > 3 & <b>bold</b>",
      site_name: "Example", fetched_at: Time.current, fetch_error: nil,
      expires_at: 1.hour.from_now
    )

    get room_url(@room)

    assert_response :success
    assert_select "##{dom_id(message, :link_embed_cards)} .link-embed-card__title",
      text: 'Tom & Jerry say "hi" <3'
    assert_includes response.body, "Tom &amp; Jerry say &quot;hi&quot; &lt;3"
    assert_not_includes response.body, "&amp;amp;"
    assert_not_includes response.body, "<b>bold</b>"
  end

  test "LinkedIn URN links render a card with the embed button" do
    stub_page("https://www.linkedin.com/feed/update/urn:li:activity:4242", <<~HTML)
      <html><head>
        <meta property="og:title" content="Jane on launching">
        <meta property="og:description" content="We shipped it.">
        <meta property="og:image" content="https://example.com/li.png">
      </head></html>
    HTML
    WebMock.stub_request(:head, "https://example.com/li.png")
      .to_return(status: 200, headers: { content_type: "image/png" })

    message = @room.messages.create!(
      creator: users(:jason), client_message_id: "embed-room-linkedin",
      markdown_source: "see https://www.linkedin.com/feed/update/urn:li:activity:4242"
    )
    perform_enqueued_jobs only: LinkEmbed::FetchJob

    get room_url(@room)

    assert_response :success
    assert_select "##{dom_id(message, :linkedin_cards)}" do
      assert_select ".linkedin-post-card", count: 1
      assert_select ".linkedin-post-card__source", text: "LinkedIn"
      assert_select ".linkedin-post-card__title", text: /Jane on launching/
      assert_select ".linkedin-post-card__excerpt", text: /We shipped it/
      assert_select "button", text: "Show embedded post", count: 1
      assert_select "[data-linkedin-embed-src-value=?]",
        "https://www.linkedin.com/embed/feed/update/urn:li:activity:4242", count: 1
      assert_select "iframe", count: 0
    end
  end

  test "login-gated LinkedIn pages render a compact chip" do
    stub_page("https://www.linkedin.com/posts/gated-post-99", "<html><head></head><body>login</body></html>")

    message = @room.messages.create!(
      creator: users(:jason), client_message_id: "embed-room-chip",
      markdown_source: "see https://www.linkedin.com/posts/gated-post-99"
    )
    perform_enqueued_jobs only: LinkEmbed::FetchJob

    get room_url(@room)

    assert_response :success
    assert_select "##{dom_id(message, :linkedin_cards)}" do
      assert_select ".linkedin-post-chip", count: 1
      assert_select ".linkedin-post-chip a[href=?]", "https://www.linkedin.com/posts/gated-post-99",
        text: "View post on LinkedIn"
      assert_select ".linkedin-post-card", count: 0
    end
  end

  test "LinkedIn posts links offer no embed button" do
    stub_page("https://www.linkedin.com/posts/open-post-100", <<~HTML)
      <html><head>
        <meta property="og:title" content="Open post">
        <meta property="og:description" content="Public excerpt.">
      </head></html>
    HTML

    message = @room.messages.create!(
      creator: users(:jason), client_message_id: "embed-room-nourn",
      markdown_source: "see https://www.linkedin.com/posts/open-post-100"
    )
    perform_enqueued_jobs only: LinkEmbed::FetchJob

    get room_url(@room)

    assert_response :success
    assert_select "##{dom_id(message, :linkedin_cards)}" do
      assert_select ".linkedin-post-card", count: 1
      assert_select "button", text: "Show embedded post", count: 0
    end
  end

  test "suppressed messages and bracketed links render empty containers" do
    suppressed = @room.messages.create!(
      creator: users(:david), client_message_id: "embed-room-suppressed",
      markdown_source: "read https://example.com/quiet"
    )
    fresh_embed("https://example.com/quiet", title: "Quiet")
    suppressed.update!(embeds_suppressed: true)

    bracketed = @room.messages.create!(
      creator: users(:david), client_message_id: "embed-room-bracketed",
      markdown_source: "read <https://example.com/loud>"
    )

    get room_url(@room)

    assert_response :success
    assert_select "##{dom_id(suppressed, :link_embed_cards)} .link-embed-card", count: 0
    assert_select "##{dom_id(bracketed, :link_embed_cards)} .link-embed-card", count: 0
    assert_empty bracketed.reload.link_embeds
  end

  test "editing a message re-syncs its embeds" do
    message = @room.messages.create!(
      creator: users(:david), client_message_id: "embed-room-edit",
      markdown_source: "read https://example.com/before-edit"
    )
    assert_equal [ "https://example.com/before-edit" ], message.link_embeds.map(&:normalized_url)

    patch room_message_url(@room, message),
      params: { message: { markdown_source: "read https://example.com/after-edit" } }

    assert_response :redirect
    assert_equal [ "https://example.com/after-edit" ], message.reload.link_embeds.map(&:normalized_url)

    get room_url(@room)
    assert_response :success
    assert_select "##{dom_id(message, :link_embed_cards)}"
  end

  test "cards in different rooms link to each message's own URL, never the first poster's" do
    first = @room.messages.create!(
      creator: users(:jason), client_message_id: "embed-leak-first",
      markdown_source: "diagram https://excalidraw.com/#json=ROOMONE,KEYONE"
    )
    other_room = rooms(:hq)
    second = other_room.messages.create!(
      creator: users(:jason), client_message_id: "embed-leak-second",
      markdown_source: "diagram https://excalidraw.com/#json=ROOMTWO,KEYTWO"
    )
    assert_equal first.link_embeds.first.id, second.link_embeds.first.id
    fresh_embed("https://excalidraw.com/", title: "Excalidraw")

    get room_url(@room)
    assert_response :success
    assert_select "##{dom_id(first, :link_embed_cards)} .link-embed-card__title a[href=?]",
      "https://excalidraw.com/#json=ROOMONE,KEYONE"

    get room_url(other_room)
    assert_response :success
    assert_select "##{dom_id(second, :link_embed_cards)} .link-embed-card__title a[href=?]",
      "https://excalidraw.com/#json=ROOMTWO,KEYTWO"
    assert_not_includes response.body, "ROOMONE"
  end

  test "LinkedIn cards in different rooms link to each message's own URL" do
    first = @room.messages.create!(
      creator: users(:jason), client_message_id: "embed-leak-li-first",
      markdown_source: "see https://www.linkedin.com/feed/update/urn:li:activity:424242#room-one"
    )
    other_room = rooms(:hq)
    second = other_room.messages.create!(
      creator: users(:jason), client_message_id: "embed-leak-li-second",
      markdown_source: "see https://www.linkedin.com/feed/update/urn:li:activity:424242#room-two"
    )
    assert_equal first.link_embeds.first.id, second.link_embeds.first.id

    get room_url(@room)
    assert_response :success
    assert_select "##{dom_id(first, :linkedin_cards)} .linkedin-post-chip a[href=?]",
      "https://www.linkedin.com/feed/update/urn:li:activity:424242#room-one"

    get room_url(other_room)
    assert_response :success
    assert_select "##{dom_id(second, :linkedin_cards)} .linkedin-post-chip a[href=?]",
      "https://www.linkedin.com/feed/update/urn:li:activity:424242#room-two"
    assert_not_includes response.body, "room-one"
  end

  test "room view query count does not grow with embedded messages" do
    2.times { |index| create_embedded_message("embed-query-small-#{index}", "https://example.com/small-#{index}") }
    get room_url(@room) # warm process-level caches before counting
    assert_response :success
    # Identical icon-cache state per leg: the custom-icon stamp query
    # re-fires on a one-second monotonic TTL, which a slow gap between
    # the legs would otherwise trip.
    Icons.expire_custom_cache!
    small = count_queries { get room_url(@room) }
    assert_response :success

    4.times { |index| create_embedded_message("embed-query-large-#{index}", "https://example.com/large-#{index}") }
    Icons.expire_custom_cache!
    large = count_queries { get room_url(@room) }
    assert_response :success

    assert_equal small, large, "room view should be O(1) in queries, got #{small} then #{large}"
  end

  private
    def stub_page(url, body)
      WebMock.stub_request(:get, url).to_return(status: 200, body: body, headers: { content_type: "text/html" })
    end

    def fresh_embed(normalized_url, title:)
      LinkEmbed.find_by!(normalized_url: normalized_url).update!(
        title: title, description: "#{title} description.", site_name: "Example",
        fetched_at: Time.current, fetch_error: nil, expires_at: 1.hour.from_now
      )
    end

    def create_embedded_message(client_message_id, url)
      message = @room.messages.create!(
        creator: users(:jason), client_message_id: client_message_id, markdown_source: "read #{url}"
      )
      fresh_embed(LinkEmbed.normalize_url(url), title: "Title for #{url}")
      message
    end

    def count_queries
      count = 0
      subscription = ActiveSupport::Notifications.subscribe("sql.active_record") do |*, payload|
        count += 1 unless payload[:name] == "SCHEMA" || payload[:cached]
      end

      ActiveRecord::Base.connection_pool.clear_query_cache
      yield
      count
    ensure
      ActiveSupport::Notifications.unsubscribe(subscription)
    end

    def dom_id(record, prefix)
      ActionView::RecordIdentifier.dom_id(record, prefix)
    end
end
