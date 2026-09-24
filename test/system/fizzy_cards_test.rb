require "application_system_test_case"

class FizzyCardsTest < ApplicationSystemTestCase
  include FizzyTestHelper
  include WebMockSystemTestHelper

  test "a linked card renders its content in the room" do
    room = rooms(:designers)
    user = users(:jz)
    FizzyConnectedAccount.create!(user: user, access_token: "user-token-system",
      fizzy_account_id: "897362094", fizzy_account_name: "Smart Data",
      fizzy_user_id: "03user1", fizzy_user_name: "JZ")

    message = room.messages.create!(
      creator: user,
      markdown_source: "track https://app.fizzy.do/897362094/cards/579",
      client_message_id: "system-fizzy-card"
    )
    card = message.fizzy_cards.first
    Fizzy::CardCache.for_viewer(card: card, user: user)
      .update!(payload: fizzy_card_payload, fetched_at: Time.current)

    sign_in "jz@37signals.com"
    join_room room

    within "##{dom_id(message)}" do
      assert_selector ".fizzy-card__title", text: "Fix the billing bug", wait: 10
      assert_selector ".fizzy-card__board", text: "Engineering"
      assert_selector ".fizzy-card__state", text: "In Progress"
      assert_selector ".fizzy-card__steps", text: "1/2 steps"
      assert_selector ".fizzy-card__link", text: "View in Fizzy"
    end
  end

  test "a message becomes a Fizzy card from the actions menu" do
    room = rooms(:designers)
    user = users(:jz)
    FizzyConnectedAccount.create!(user: user, access_token: "user-token-system",
      fizzy_account_id: "897362094", fizzy_account_name: "Smart Data",
      fizzy_user_id: "03user1", fizzy_user_name: "JZ")

    message = room.messages.create!(
      creator: user,
      markdown_source: "The deploy is broken",
      client_message_id: "system-fizzy-create"
    )

    stub_request(:get, "https://app.fizzy.do/897362094/boards.json")
      .to_return(status: 200, body: [ fizzy_board_payload ].to_json)
    create = stub_request(:post, "https://app.fizzy.do/897362094/boards/03board1/cards.json")
      .to_return(status: 201, body: fizzy_card_payload(number: 580, title: "The deploy is broken").to_json)
    stub_fizzy_card(580, token: "user-token-system",
      payload: fizzy_card_payload(number: 580, title: "The deploy is broken"))

    sign_in "jz@37signals.com"
    join_room room

    within_message(message) do
      right_click_message
    end
    assert_message_menu_open
    click_link "Create Fizzy card"

    assert_selector "h1", text: "Create Fizzy card", wait: 10
    assert_field "Title", with: "The deploy is broken"
    select "Engineering", from: "Board"
    click_button "Create card"

    assert_selector ".flash", text: "Fizzy card #580 created", wait: 10
    assert_requested create
    assert_message_text "https://app.fizzy.do/897362094/cards/580"
  end
end
