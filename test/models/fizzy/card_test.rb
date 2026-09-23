require "test_helper"

class Fizzy::CardTest < ActiveSupport::TestCase
  include FizzyTestHelper

  test "for_reference finds or creates by account and number" do
    card = Fizzy::Card.for_reference(account_id: "897362094", number: 579)

    assert_equal card, Fizzy::Card.for_reference(account_id: "897362094", number: 579)
    assert_not_equal card, Fizzy::Card.for_reference(account_id: "6264925", number: 579)
  end

  test "web_url builds the Fizzy link" do
    card = Fizzy::Card.for_reference(account_id: "897362094", number: 579)

    assert_equal "https://app.fizzy.do/897362094/cards/579", card.web_url
  end

  test "web_url follows the configured Fizzy host" do
    original = ENV["FIZZY_API_BASE_URL"]
    ENV["FIZZY_API_BASE_URL"] = "https://fizzy.example.com"
    begin
      card = Fizzy::Card.for_reference(account_id: "897362094", number: 579)

      assert_equal "https://fizzy.example.com/897362094/cards/579", card.web_url

      ENV["FIZZY_API_BASE_URL"] = "https://fizzy.example.com/"
      assert_equal "https://fizzy.example.com/897362094/cards/579", card.web_url
    ensure
      ENV["FIZZY_API_BASE_URL"] = original
    end
  end

  test "card cache is per viewer and reports staleness" do
    card = Fizzy::Card.for_reference(account_id: "897362094", number: 579)
    cache = Fizzy::CardCache.for_viewer(card: card, user: users(:david))

    assert cache.stale?
    assert_equal cache, Fizzy::CardCache.for_viewer(card: card, user: users(:david))
    assert_not_equal cache, Fizzy::CardCache.for_viewer(card: card, user: users(:jz))

    cache.update!(payload: fizzy_card_payload, fetched_at: Time.current)
    assert_not cache.reload.stale?

    travel_to 6.minutes.from_now do
      assert cache.reload.stale?
    end
  end

  test "claim admits one fetch per viewer and card per window" do
    card = Fizzy::Card.for_reference(account_id: "897362094", number: 579)
    cache = Fizzy::CardCache.for_viewer(card: card, user: users(:david))

    assert cache.claim_fetch_request!
    assert_not cache.claim_fetch_request!

    cache.release_fetch_request!
    assert cache.claim_fetch_request!
  end

  test "posting a message references its cards and warms the author's cache" do
    link_fizzy!(users(:david), token: "david-token")
    stub_fizzy_card(579, token: "david-token")

    message = nil
    assert_enqueued_with(job: Fizzy::FetchCardJob) do
      message = rooms(:watercooler).messages.create!(
        creator: users(:david),
        markdown_source: "track https://app.fizzy.do/897362094/cards/579",
        client_message_id: "fizzy-ref-sync-1"
      )
    end

    card = Fizzy::Card.find_by!(account_id: "897362094", number: 579)
    assert_equal [ card ], message.reload.fizzy_cards

    perform_enqueued_jobs only: Fizzy::FetchCardJob
    cache = Fizzy::CardCache.find_by!(fizzy_card_id: card.id, user_id: users(:david).id)
    assert_equal "Fix the billing bug", cache.payload["title"]
  end

  test "posting without a linked account references without fetching" do
    message = rooms(:watercooler).messages.create!(
      creator: users(:jz),
      markdown_source: "track https://app.fizzy.do/897362094/cards/579",
      client_message_id: "fizzy-ref-sync-2"
    )

    assert_equal 579, message.reload.fizzy_cards.first.number
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "editing a message to remove the URL clears its references" do
    message = rooms(:watercooler).messages.create!(
      creator: users(:jz),
      markdown_source: "track https://app.fizzy.do/897362094/cards/579",
      client_message_id: "fizzy-ref-sync-3"
    )
    assert_equal 1, message.fizzy_cards.count

    message.update!(markdown_source: "no link here")

    assert_empty message.reload.fizzy_cards
  end
end
