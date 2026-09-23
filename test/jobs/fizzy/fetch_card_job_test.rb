require "test_helper"

class Fizzy::FetchCardJobTest < ActiveSupport::TestCase
  include FizzyTestHelper
  include ActionCable::TestHelper

  setup do
    @card = Fizzy::Card.for_reference(account_id: "897362094", number: 579)
  end

  test "success stores the payload and broadcasts fresh frames" do
    link_fizzy!(users(:david), token: "david-token")
    stub_fizzy_card(579, token: "david-token")
    message = post_card_message

    assert_broadcasts room_messages_stream_name(message.room), 1 do
      Fizzy::FetchCardJob.perform_now(@card, users(:david))
    end

    cache = Fizzy::CardCache.find_by!(fizzy_card_id: @card.id, user_id: users(:david).id)
    assert_equal "Fix the billing bug", cache.payload["title"]
    assert_nil cache.fetch_error
    assert_not_nil cache.fetched_at
  end

  test "a 404 stores not-found so the frame renders the chip" do
    link_fizzy!(users(:jz), token: "jz-token")
    stub_request(:get, "https://app.fizzy.do/897362094/cards/579.json").to_return(status: 404, body: {}.to_json)

    Fizzy::FetchCardJob.perform_now(@card, users(:jz))

    cache = Fizzy::CardCache.find_by!(fizzy_card_id: @card.id, user_id: users(:jz).id)
    assert_predicate cache, :not_found?
    assert_nil cache.payload
  end

  test "a 403 stores not-found so the frame renders the chip" do
    link_fizzy!(users(:jz), token: "jz-token")
    stub_request(:get, "https://app.fizzy.do/897362094/cards/579.json")
      .to_return(status: 403, body: { error: "no access" }.to_json)

    Fizzy::FetchCardJob.perform_now(@card, users(:jz))

    cache = Fizzy::CardCache.find_by!(fizzy_card_id: @card.id, user_id: users(:jz).id)
    assert_predicate cache, :not_found?
    assert_nil cache.payload
  end

  test "a 401 disconnects the account and clears the cache row" do
    account = link_fizzy!(users(:david), token: "david-token")
    stub_request(:get, "https://app.fizzy.do/897362094/cards/579.json").to_return(status: 401, body: {}.to_json)

    Fizzy::FetchCardJob.perform_now(@card, users(:david))

    assert_not account.reload.connected?
    cache = Fizzy::CardCache.find_by!(fizzy_card_id: @card.id, user_id: users(:david).id)
    assert_nil cache.payload
    assert_nil cache.fetched_at
  end

  test "a transport failure stores a renderable error" do
    link_fizzy!(users(:david), token: "david-token")
    stub_request(:get, "https://app.fizzy.do/897362094/cards/579.json").to_timeout

    Fizzy::FetchCardJob.perform_now(@card, users(:david))

    cache = Fizzy::CardCache.find_by!(fizzy_card_id: @card.id, user_id: users(:david).id)
    assert_includes cache.fetch_error, "Could not reach Fizzy"
    assert_not_nil cache.fetched_at
  end

  test "no usable account makes no request" do
    Fizzy::FetchCardJob.perform_now(@card, users(:kevin))

    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  private
    def post_card_message
      rooms(:watercooler).messages.create!(
        creator: users(:kevin),
        markdown_source: "track https://app.fizzy.do/897362094/cards/579",
        client_message_id: "fizzy-fetch-job-1"
      )
    end

    def room_messages_stream_name(room)
      Turbo::StreamsChannel.signed_stream_name([ room, :messages ]).then { |signed| Turbo::StreamsChannel.verified_stream_name(signed) }
    end
end
