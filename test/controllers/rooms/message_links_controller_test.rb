require "test_helper"

class Rooms::MessageLinksControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @room = rooms(:designers)
    @other_room = rooms(:watercooler)
    @source = @other_room.messages.create!(
      body: "cross-room quoted words", client_message_id: "quote-source",
      creator: users(:jason)
    )
    @quote = @room.messages.create!(
      markdown_source: "look over here /rooms/#{@other_room.id}/@#{@source.id}",
      client_message_id: "quote-quoting", creator: users(:david)
    )
    @reference = @quote.message_references.first
  end

  test "a member of the source room sees the quote card" do
    get room_message_link_url(@room, @reference)

    assert_response :success
    assert_select "blockquote.message-quote", text: /cross-room quoted words/
    assert_select ".message-quote__author", text: "Jason"
    assert_select ".message-quote__room", text: /All Talk/
    assert_select "a", text: "Jump to message"
  end

  test "a non-member of the source room sees only the private chip" do
    sign_in :kevin

    get room_message_link_url(@room, @reference)

    assert_response :success
    assert_select ".message-quote-private", text: "Message in a private room"
    assert_not_includes response.body, "cross-room quoted words"
    assert_select ".message-quote", count: 0
  end

  test "a non-member of the quoting room gets nothing" do
    sign_in :kevin

    get room_message_link_url(rooms(:watercooler), @reference)

    assert_response :not_found
  end

  test "a reference from another room gets nothing" do
    other_quote = @other_room.messages.create!(
      markdown_source: "elsewhere /rooms/#{@other_room.id}/@#{@source.id}",
      client_message_id: "quote-elsewhere", creator: users(:david)
    )

    get room_message_link_url(@room, other_quote.message_references.first)

    assert_response :not_found
  end

  test "a same-room quote renders inline in the room" do
    same_source = @room.messages.create!(
      body: "same-room quoted words", client_message_id: "quote-same-source", creator: users(:jz)
    )
    @room.messages.create!(
      markdown_source: "right here /rooms/#{@room.id}/@#{same_source.id}",
      client_message_id: "quote-same", creator: users(:david)
    )

    get room_url(@room)

    assert_response :success
    assert_select "blockquote.message-quote", text: /same-room quoted words/
  end

  test "a cross-room quote renders a lazy frame in the room" do
    get room_url(@room)

    assert_response :success
    assert_select "turbo-frame.message-link-frame[src=?]", room_message_link_path(@room, @reference)
  end

  test "editing the source enqueues a job that refreshes quoting cards over the stream" do
    sign_in :jason
    assert_enqueued_with(job: Message::QuoteCardsRefreshJob, args: [ @source.id ]) do
      patch room_message_url(@other_room, @source),
        params: { message: { body: "cross-room quoted words, revised" } }
    end

    assert_response :redirect

    perform_enqueued_jobs only: Message::QuoteCardsRefreshJob

    assert_rendered_turbo_stream_broadcast @room, :messages,
      action: "replace", target: [ @quote, :message_link_cards ] do |stream|
      assert_select stream, "turbo-frame.message-link-frame"
    end
  end

  test "deleting the source clears quoting cards and busts their cache" do
    @quote.update_columns(updated_at: 1.day.ago)

    delete room_message_url(@other_room, @source, format: :turbo_stream)

    assert_response :success
    assert_empty MessageReference.where(id: @reference.id)
    assert @quote.reload.updated_at > 1.day.ago
    assert_rendered_turbo_stream_broadcast @room, :messages,
      action: "replace", target: [ @quote, :message_link_cards ]
  end

  test "two viewers of a cached direct-room quote see the same neutral label" do
    dm = rooms(:david_and_jason)
    source = dm.messages.create!(
      body: "cached dm source words", client_message_id: "dm-quote-source", creator: users(:david)
    )
    dm.messages.create!(
      markdown_source: "see /rooms/#{dm.id}/@#{source.id}",
      client_message_id: "dm-quote-quoting", creator: users(:david)
    )

    with_caching do
      sign_in :david
      get room_messages_url(dm)
      assert_response :success
      david_labels = quote_room_labels(response.body)

      sign_in :jason
      get room_messages_url(dm)
      assert_response :success
      jason_labels = quote_room_labels(response.body)

      assert_equal [ "in a direct message" ], david_labels
      assert_equal david_labels, jason_labels
    end
  end

  test "quote cards cost the same queries regardless of card count" do
    one = [ create_dm_quote(users(:jason), "qc-one") ]
    small = count_card_render_queries(one)

    many = one + [ create_dm_quote(users(:kevin), "qc-two"), create_dm_quote(users(:jz), "qc-three") ]
    large = count_card_render_queries(many)

    assert_equal 0, small
    assert_equal small, large
  end

  test "editing a message to add a permalink replaces its own card container" do
    plain = @room.messages.create!(
      body: "nothing quoted yet", client_message_id: "quote-plain", creator: users(:david)
    )

    patch room_message_url(@room, plain),
      params: { message: { markdown_source: "now quoting /rooms/#{@room.id}/@#{@source.id}" } }

    assert_response :redirect
    assert_equal [ @source ], plain.reload.referenced_messages
    assert_rendered_turbo_stream_broadcast @room, :messages,
      action: "replace", target: [ plain, :message_link_cards ]
  end

  private
    def quote_room_labels(html)
      Nokogiri::HTML(html).css(".message-quote__room").map { |node| node.text.squish }
    end

    # A same-room quote inside its own direct room, so every card's room
    # lookup carries distinct binds and can never hide in the query cache.
    def create_dm_quote(peer, seq)
      dm = with_current_user(users(:david)) do
        Rooms::Direct.find_or_create_for([ users(:david), peer ])
      end
      source = dm.messages.create!(
        body: "dm card source #{seq}", client_message_id: "dm-card-source-#{seq}", creator: users(:david)
      )
      dm.messages.create!(
        markdown_source: "see /rooms/#{dm.id}/@#{source.id}",
        client_message_id: "dm-card-quote-#{seq}", creator: users(:david)
      ).id
    end

    def count_card_render_queries(quote_ids)
      quotes = Message.with_rendering_details.where(id: quote_ids).to_a
      count = 0
      subscriber = ActiveSupport::Notifications.subscribe("sql.active_record") do |*, payload|
        count += 1 unless payload[:name] == "SCHEMA" || payload[:cached]
      end

      ActiveRecord::Base.connection_pool.clear_query_cache
      with_current_user(users(:david)) do
        quotes.each do |quote|
          ApplicationController.render(partial: "messages/message_links/cards", locals: { message: quote })
        end
      end
      count
    ensure
      ActiveSupport::Notifications.unsubscribe(subscriber)
    end

    def with_current_user(user)
      previous = Current.user
      Current.user = user
      yield
    ensure
      Current.user = previous
    end

    def with_caching(&block)
      original_cache = Rails.cache
      original_collection_cache = ActionView::PartialRenderer.collection_cache
      original_perform_caching = ActionController::Base.perform_caching
      Rails.cache = ActiveSupport::Cache::MemoryStore.new
      # The collection renderer snapshots its store at boot, so point it at
      # the memory store too or cached: keeps hitting the null store.
      ActionView::PartialRenderer.collection_cache = Rails.cache
      ActionController::Base.perform_caching = true

      block.call
    ensure
      ActionController::Base.perform_caching = original_perform_caching
      ActionView::PartialRenderer.collection_cache = original_collection_cache
      Rails.cache = original_cache
    end
end
