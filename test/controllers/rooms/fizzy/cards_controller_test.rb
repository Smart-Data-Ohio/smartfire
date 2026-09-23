require "test_helper"

class Rooms::Fizzy::CardsControllerTest < ActionDispatch::IntegrationTest
  include FizzyTestHelper

  setup do
    @room = rooms(:designers)
    @message = @room.messages.create!(
      creator: users(:kevin),
      markdown_source: "track https://app.fizzy.do/897362094/cards/579",
      client_message_id: "fizzy-frame-1"
    )
    @card = @message.fizzy_cards.first
  end

  test "a viewer whose token can access the card sees it rendered" do
    link_fizzy!(users(:david), token: "david-token")
    stub_fizzy_card(579, token: "david-token")
    sign_in :david

    get room_fizzy_card_url(@room, @card, message_id: @message.id)
    assert_includes response.body, "Loading Fizzy card"

    perform_enqueued_jobs only: Fizzy::FetchCardJob

    get room_fizzy_card_url(@room, @card, message_id: @message.id)

    assert_response :success
    assert_includes response.body, "Fix the billing bug"
    assert_includes response.body, "Engineering"
    assert_includes response.body, "In Progress"
    assert_includes response.body, "David"
    assert_includes response.body, "#billing"
    assert_includes response.body, "1/2 steps"
    assert_includes response.body, "View in Fizzy"
    assert_includes response.body, "https://app.fizzy.do/897362094/cards/579"
  end

  test "a viewer Fizzy 404s sees the chip without a connect hint" do
    link_fizzy!(users(:jz), token: "jz-token")
    stub_request(:get, "https://app.fizzy.do/897362094/cards/579.json").to_return(status: 404, body: {}.to_json)
    sign_in :jz

    get room_fizzy_card_url(@room, @card, message_id: @message.id)
    assert_includes response.body, "Loading Fizzy card"

    perform_enqueued_jobs only: Fizzy::FetchCardJob

    get room_fizzy_card_url(@room, @card, message_id: @message.id)
    assert_response :success
    assert_includes response.body, "Fizzy card #579"
    assert_not_includes response.body, "Connect Fizzy to preview"
    assert_not_includes response.body, "Fix the billing bug"
  end

  test "a viewer Fizzy 403s sees the chip without a connect hint" do
    link_fizzy!(users(:jz), token: "jz-token")
    stub_request(:get, "https://app.fizzy.do/897362094/cards/579.json")
      .to_return(status: 403, body: { error: "no access" }.to_json)
    sign_in :jz

    get room_fizzy_card_url(@room, @card, message_id: @message.id)
    assert_includes response.body, "Loading Fizzy card"

    perform_enqueued_jobs only: Fizzy::FetchCardJob

    get room_fizzy_card_url(@room, @card, message_id: @message.id)
    assert_response :success
    assert_includes response.body, "Fizzy card #579"
    assert_not_includes response.body, "Connect Fizzy to preview"
    assert_not_includes response.body, "Fix the billing bug"
    assert_not_includes response.body, "Couldn’t load this Fizzy card"
  end

  test "a viewer without a connected account sees the chip with a connect hint" do
    sign_in :kevin

    get room_fizzy_card_url(@room, @card, message_id: @message.id)

    assert_response :success
    assert_includes response.body, "Fizzy card #579"
    assert_includes response.body, "Connect Fizzy to preview"
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "a stale cache renders while a refresh is enqueued once" do
    link_fizzy!(users(:david), token: "david-token")
    cache = Fizzy::CardCache.for_viewer(card: @card, user: users(:david))
    cache.update!(payload: fizzy_card_payload(title: "Stale title"), fetched_at: 6.minutes.ago)
    stub_fizzy_card(579, token: "david-token")
    sign_in :david

    assert_enqueued_with(job: Fizzy::FetchCardJob) do
      get room_fizzy_card_url(@room, @card, message_id: @message.id)
    end
    assert_includes response.body, "Stale title"

    assert_no_enqueued_jobs only: Fizzy::FetchCardJob do
      get room_fizzy_card_url(@room, @card, message_id: @message.id)
    end
  end

  test "a fresh cache renders without a fetch" do
    link_fizzy!(users(:david), token: "david-token")
    cache = Fizzy::CardCache.for_viewer(card: @card, user: users(:david))
    cache.update!(payload: fizzy_card_payload, fetched_at: Time.current)
    sign_in :david

    get room_fizzy_card_url(@room, @card, message_id: @message.id)

    assert_response :success
    assert_includes response.body, "Fix the billing bug"
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "a fetch error renders inline" do
    link_fizzy!(users(:david), token: "david-token")
    stub_request(:get, "https://app.fizzy.do/897362094/cards/579.json").to_return(status: 500, body: "boom")
    sign_in :david

    get room_fizzy_card_url(@room, @card, message_id: @message.id)
    perform_enqueued_jobs only: Fizzy::FetchCardJob
    get room_fizzy_card_url(@room, @card, message_id: @message.id)

    assert_includes response.body, "Couldn’t load this Fizzy card"
  end

  test "a closed card shows its Closed status" do
    link_fizzy!(users(:david), token: "david-token")
    stub_fizzy_card(579, token: "david-token", payload: fizzy_card_payload(closed: true, column_name: nil))
    sign_in :david

    get room_fizzy_card_url(@room, @card, message_id: @message.id)
    perform_enqueued_jobs only: Fizzy::FetchCardJob
    get room_fizzy_card_url(@room, @card, message_id: @message.id)

    assert_includes response.body, "Closed"
  end

  test "a non-member gets 404" do
    link_fizzy!(users(:david), token: "david-token")
    sign_in :david
    rooms(:designers).memberships.where(user: users(:david)).delete_all

    # RoomScoped raises RecordNotFound, which renders 404 outside tests.
    assert_raises(ActiveRecord::RecordNotFound) do
      get room_fizzy_card_url(@room, @card, message_id: @message.id)
    end
  end

  test "a card not referenced by the message gets 404" do
    link_fizzy!(users(:david), token: "david-token")
    other = Fizzy::Card.for_reference(account_id: "897362094", number: 1)
    sign_in :david

    assert_raises(ActiveRecord::RecordNotFound) do
      get room_fizzy_card_url(@room, other, message_id: @message.id)
    end
  end
end
