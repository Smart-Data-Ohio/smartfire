require "test_helper"

class Message::QuoteCardsRefreshJobTest < ActiveJob::TestCase
  include Github::PullRequestsHelper
  include MessageLinksHelper

  setup do
    @source_room = rooms(:watercooler)
    @source = @source_room.messages.create!(
      body: "refresh job source words", client_message_id: "refresh-source", creator: users(:jason)
    )
  end

  test "refreshes every quoting card across rooms" do
    quotes = [ rooms(:designers), rooms(:pets), rooms(:hq) ].map.with_index do |room, index|
      room.messages.create!(
        markdown_source: "quoting /rooms/#{@source_room.id}/@#{@source.id}",
        client_message_id: "refresh-quote-#{index}", creator: users(:david)
      )
    end

    Message::QuoteCardsRefreshJob.perform_now(@source.id)

    quotes.each do |quote|
      assert_includes card_replace_targets(quote.room), ActionView::RecordIdentifier.dom_id(quote, :message_link_cards)
    end
  end

  test "iterates in batches" do
    3.times do |index|
      rooms(:designers).messages.create!(
        markdown_source: "quoting /rooms/#{@source_room.id}/@#{@source.id}",
        client_message_id: "refresh-batch-#{index}", creator: users(:david)
      )
    end

    Message::QuoteCardsRefreshJob.perform_now(@source.id, batch_size: 2)

    assert_equal 3, card_replace_broadcast_count(rooms(:designers))
  end

  test "stops at the cap" do
    3.times do |index|
      rooms(:designers).messages.create!(
        markdown_source: "quoting /rooms/#{@source_room.id}/@#{@source.id}",
        client_message_id: "refresh-cap-#{index}", creator: users(:david)
      )
    end

    Message::QuoteCardsRefreshJob.perform_now(@source.id, max_messages: 2)

    assert_equal 2, card_replace_broadcast_count(rooms(:designers))
  end

  test "quotes beyond the cap refresh on the next load through the cache key" do
    quote = rooms(:designers).messages.create!(
      markdown_source: "quoting /rooms/#{@source_room.id}/@#{@source.id}",
      client_message_id: "refresh-fallback", creator: users(:david)
    )
    before = message_with_pr_cards_cache_key(Message.with_rendering_details.find(quote.id))

    travel 1.minute do
      @source.update!(body: "refresh job source words, revised")
    end

    assert_not_equal before, message_with_pr_cards_cache_key(Message.with_rendering_details.find(quote.id))
  end

  test "a missing source is a no-op" do
    Message::QuoteCardsRefreshJob.perform_now(-1)

    assert_equal 0, card_replace_broadcast_count(rooms(:designers))
  end

  private
    def card_replace_broadcast_count(room)
      card_replace_fragment(room).css("turbo-stream[action=\"replace\"]").size
    end

    def card_replace_targets(room)
      card_replace_fragment(room).css("turbo-stream[action=\"replace\"]").map { |node| node["target"] }
    end

    def card_replace_fragment(room)
      streams = ActionCable.server.pubsub.broadcasts("#{room.to_gid_param}:messages")
      Nokogiri::HTML.fragment(streams.map { |json| JSON.parse(json) }.join("\n\n"))
    end
end
