require "test_helper"

class Rooms::Fizzy::MessageCardsControllerTest < ActionDispatch::IntegrationTest
  include FizzyTestHelper

  setup do
    @room = rooms(:designers)
    @message = @room.messages.create!(
      creator: users(:kevin),
      markdown_source: "The deploy is broken\nWe should track a fix for the deploy script.",
      client_message_id: "fizzy-create-1"
    )
  end

  test "new lists the member's boards with the message prefilled" do
    token = "david-token"
    link_fizzy!(users(:david), token: token)
    stub = stub_request(:get, "https://app.fizzy.do/897362094/boards.json")
      .with(headers: { "Authorization" => "Bearer #{token}" })
      .to_return(status: 200, body: [ fizzy_board_payload, fizzy_board_payload(id: "03board2", name: "Support") ].to_json)
    sign_in :david

    get new_room_message_fizzy_card_url(@room, @message)

    assert_response :success
    assert_requested stub
    assert_includes response.body, "Engineering"
    assert_includes response.body, "Support"
    assert_includes response.body, "The deploy is broken"
    assert_includes response.body, "We should track a fix for the deploy script."
  end

  test "new without a linked account shows a connect prompt" do
    sign_in :david

    get new_room_message_fizzy_card_url(@room, @message)

    assert_response :success
    assert_includes response.body, "Connect Fizzy on your profile"
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "new when Fizzy is unreachable redirects with an alert" do
    link_fizzy!(users(:david), token: "david-token")
    stub_request(:get, "https://app.fizzy.do/897362094/boards.json").to_timeout
    sign_in :david

    get new_room_message_fizzy_card_url(@room, @message)

    assert_redirected_to room_path(@room)
    assert_equal "Could not reach Fizzy. Try again.", flash[:alert]
  end

  test "new when the token is rejected disconnects and redirects to the profile" do
    account = link_fizzy!(users(:david), token: "david-token")
    stub_request(:get, "https://app.fizzy.do/897362094/boards.json").to_return(status: 401, body: {}.to_json)
    sign_in :david

    get new_room_message_fizzy_card_url(@room, @message)

    assert_redirected_to user_profile_path
    assert_not account.reload.connected?
  end

  test "create posts the card and replies with its unfurl" do
    link_fizzy!(users(:david), token: "david-token")
    stub = stub_request(:post, "https://app.fizzy.do/897362094/boards/03board1/cards.json")
      .with(body: { card: { title: "The deploy is broken", description: "Fix it\n\nSource: x" } }.to_json)
      .to_return(status: 201, body: fizzy_card_payload(number: 580, title: "The deploy is broken").to_json)
    # The reply's own unfurl fetch runs with the author's token.
    stub_fizzy_card(580, token: "david-token", payload: fizzy_card_payload(number: 580, title: "The deploy is broken"))
    sign_in :david

    assert_difference -> { @room.messages.count }, 1 do
      post room_message_fizzy_cards_url(@room, @message),
        params: { board_id: "03board1", title: "The deploy is broken", description: "Fix it\n\nSource: x" }
    end

    assert_requested stub
    assert_redirected_to room_path(@room)
    assert_equal "Fizzy card #580 created.", flash[:notice]

    reply = @room.messages.order(:id).last
    assert_equal @message, reply.reply_to_message
    assert_nil reply.thread_id
    assert_includes reply.plain_text_body, "https://app.fizzy.do/897362094/cards/580"
    assert_equal 580, reply.fizzy_cards.first.number
  end

  test "create from a thread message replies in the same thread" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), parent_message: @message)
    ThreadMembership.join!(thread, users(:david))
    thread_message = thread.post_message!(creator: users(:david),
      attributes: { markdown_source: "Threaded problem", client_message_id: "fizzy-create-thread-1" })
    link_fizzy!(users(:david), token: "david-token")
    stub_request(:post, "https://app.fizzy.do/897362094/boards/03board1/cards.json")
      .to_return(status: 201, body: fizzy_card_payload(number: 581).to_json)
    stub_fizzy_card(581, token: "david-token", payload: fizzy_card_payload(number: 581))
    sign_in :david

    post room_thread_message_fizzy_cards_url(@room, thread, thread_message),
      params: { board_id: "03board1", title: "Threaded problem", description: "Details" }

    assert_redirected_to room_thread_path(@room, thread)

    reply = thread.messages.order(:id).last
    assert_equal thread_message, reply.reply_to_message
    assert_includes reply.plain_text_body, "https://app.fizzy.do/897362094/cards/581"
  end

  test "create without a board or title re-renders with an alert" do
    link_fizzy!(users(:david), token: "david-token")
    stub_request(:get, "https://app.fizzy.do/897362094/boards.json")
      .to_return(status: 200, body: [ fizzy_board_payload ].to_json)
    sign_in :david

    post room_message_fizzy_cards_url(@room, @message), params: { board_id: "", title: "x" }

    assert_response :unprocessable_entity
    assert_equal "Choose a board and enter a title.", flash.now[:alert]
    assert_not_requested :post, %r{app\.fizzy\.do}
  end

  test "create with a read-only token explains without disconnecting" do
    account = link_fizzy!(users(:david), token: "david-token")
    stub_request(:post, "https://app.fizzy.do/897362094/boards/03board1/cards.json")
      .to_return(status: 401, body: {}.to_json)
    stub_fizzy_identity("david-token")
    sign_in :david

    post room_message_fizzy_cards_url(@room, @message),
      params: { board_id: "03board1", title: "Title", description: "Body" }

    assert_redirected_to room_path(@room)
    assert_includes flash[:alert], "read-only"
    assert_predicate account.reload, :connected?
  end

  test "create with a revoked token disconnects and redirects to the profile" do
    account = link_fizzy!(users(:david), token: "david-token")
    stub_request(:post, "https://app.fizzy.do/897362094/boards/03board1/cards.json")
      .to_return(status: 401, body: {}.to_json)
    stub_request(:get, "https://app.fizzy.do/my/identity.json").to_return(status: 401, body: {}.to_json)
    sign_in :david

    post room_message_fizzy_cards_url(@room, @message),
      params: { board_id: "03board1", title: "Title", description: "Body" }

    assert_redirected_to user_profile_path
    assert_not account.reload.connected?
    assert_equal 0, @room.messages.where("markdown_source LIKE ?", "%fizzy.do%").count
  end

  test "create when the permission probe fails stays connected without a reply" do
    account = link_fizzy!(users(:david), token: "david-token")
    stub_request(:post, "https://app.fizzy.do/897362094/boards/03board1/cards.json")
      .to_return(status: 401, body: {}.to_json)
    stub_request(:get, "https://app.fizzy.do/my/identity.json").to_return(status: 500, body: "boom")
    sign_in :david

    assert_no_difference -> { @room.messages.count } do
      post room_message_fizzy_cards_url(@room, @message),
        params: { board_id: "03board1", title: "Title", description: "Body" }
    end

    assert_redirected_to room_path(@room)
    assert_includes flash[:alert], "Could not reach Fizzy"
    assert_predicate account.reload, :connected?
  end

  test "create in a locked thread is refused" do
    thread = ChannelThread.create!(room: @room, creator: users(:david), parent_message: @message)
    ThreadMembership.join!(thread, users(:david))
    thread_message = thread.post_message!(creator: users(:david),
      attributes: { markdown_source: "Locked problem", client_message_id: "fizzy-create-locked-1" })
    thread.update!(locked_at: Time.current)
    link_fizzy!(users(:david), token: "david-token")
    stub = stub_request(:post, "https://app.fizzy.do/897362094/boards/03board1/cards.json")
      .to_return(status: 201, body: fizzy_card_payload(number: 582).to_json)
    sign_in :david

    post room_thread_message_fizzy_cards_url(@room, thread, thread_message),
      params: { board_id: "03board1", title: "Locked problem", description: "Details" }

    assert_redirected_to room_thread_path(@room, thread)
    assert_includes flash[:alert], "locked"
    assert_not_requested stub
  end

  test "a non-member cannot open the form" do
    link_fizzy!(users(:david), token: "david-token")
    sign_in :david
    rooms(:designers).memberships.where(user: users(:david)).delete_all

    # RoomScoped raises RecordNotFound, which renders 404 outside tests.
    assert_raises(ActiveRecord::RecordNotFound) do
      get new_room_message_fizzy_card_url(@room, @message)
    end
  end
end
