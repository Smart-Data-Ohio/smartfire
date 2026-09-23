require "test_helper"

class Fizzy::ClientTest < ActiveSupport::TestCase
  include FizzyTestHelper

  setup do
    @token = "user-token-123"
    @client = Fizzy::Client.new(token: @token)
  end

  test "identity returns the token's accounts" do
    stub = stub_fizzy_identity(@token)

    identity = @client.identity

    assert_requested stub
    assert_equal "Smart Data", identity["accounts"].first["name"]
    assert_equal "David", identity["accounts"].first["user"]["name"]
  end

  test "identity raises Unauthorized on 401" do
    stub_request(:get, "https://app.fizzy.do/my/identity.json").to_return(status: 401, body: {}.to_json)

    assert_raises(Fizzy::Client::Unauthorized) { Fizzy::Client.identity_for("bad-token") }
  end

  test "boards lists boards for the account" do
    stub = stub_request(:get, "https://app.fizzy.do/897362094/boards.json")
      .with(headers: { "Authorization" => "Bearer #{@token}" })
      .to_return(status: 200, body: [ fizzy_board_payload ].to_json)

    assert_equal [ "Engineering" ], @client.boards("897362094").map { |board| board["name"] }
    assert_requested stub
  end

  test "card returns the card with steps" do
    stub = stub_fizzy_card(579, token: @token)

    card = @client.card("897362094", 579)

    assert_requested stub
    assert_equal "Fix the billing bug", card["title"]
    assert_equal 2, card["steps"].size
  end

  test "search queries the search endpoint" do
    stub = stub_request(:get, "https://app.fizzy.do/897362094/search.json?q=billing%20bug")
      .to_return(status: 200, body: [ fizzy_card_payload ].to_json)

    assert_equal [ 579 ], @client.search("897362094", "billing bug").map { |card| card["number"] }
    assert_requested stub
  end

  test "create_card posts title and description and returns the card" do
    stub = stub_request(:post, "https://app.fizzy.do/897362094/boards/03board1/cards.json")
      .with(body: { card: { title: "New card", description: "From chat" } }.to_json)
      .to_return(status: 201, body: fizzy_card_payload(number: 580, title: "New card").to_json)

    card = @client.create_card("897362094", "03board1", title: "New card", description: "From chat")

    assert_requested stub
    assert_equal 580, card["number"]
  end

  test "create_comment posts the comment body" do
    stub = stub_request(:post, "https://app.fizzy.do/897362094/cards/579/comments.json")
      .with(body: { comment: { body: "Nice work" } }.to_json)
      .to_return(status: 201, body: fizzy_comment_payload.to_json)

    comment = @client.create_comment("897362094", 579, body: "Nice work")

    assert_requested stub
    assert_equal "https://app.fizzy.do/897362094/cards/579/comments/03comment1", comment["url"]
  end

  test "move_to_column posts the triage endpoint" do
    stub = stub_request(:post, "https://app.fizzy.do/897362094/cards/579/triage.json")
      .with(body: { column_id: "03column1" }.to_json)
      .to_return(status: 204)

    assert @client.move_to_column("897362094", 579, column_id: "03column1")
    assert_requested stub
  end

  test "close and reopen hit the closure endpoint" do
    close = stub_request(:post, "https://app.fizzy.do/897362094/cards/579/closure.json").to_return(status: 204)
    reopen = stub_request(:delete, "https://app.fizzy.do/897362094/cards/579/closure.json").to_return(status: 204)

    assert @client.close_card("897362094", 579)
    assert @client.reopen_card("897362094", 579)
    assert_requested close
    assert_requested reopen
  end

  test "a 404 raises NotFound" do
    stub_request(:get, "https://app.fizzy.do/897362094/cards/404.json").to_return(status: 404, body: {}.to_json)

    assert_raises(Fizzy::Client::NotFound) { @client.card("897362094", 404) }
  end

  test "a 403 raises Refused with Fizzy's message" do
    stub_request(:get, "https://app.fizzy.do/897362094/cards/579.json")
      .to_return(status: 403, body: { error: "no access" }.to_json)

    error = assert_raises(Fizzy::Client::Refused) { @client.card("897362094", 579) }
    assert_equal "Fizzy refused: no access", error.message
  end

  test "a 422 raises Refused" do
    stub_request(:post, "https://app.fizzy.do/897362094/boards/03board1/cards.json")
      .to_return(status: 422, body: { message: "Title can't be blank" }.to_json)

    error = assert_raises(Fizzy::Client::Refused) do
      @client.create_card("897362094", "03board1", title: "")
    end
    assert_equal "Fizzy refused: Title can't be blank", error.message
  end

  test "a 500 raises Error" do
    stub_request(:get, "https://app.fizzy.do/897362094/boards.json").to_return(status: 500, body: "boom")

    error = assert_raises(Fizzy::Client::Error) { @client.boards("897362094") }
    assert_equal "Fizzy returned 500", error.message
  end

  test "a transport failure raises Error" do
    stub_request(:get, "https://app.fizzy.do/897362094/boards.json").to_timeout

    assert_raises(Fizzy::Client::Error) { @client.boards("897362094") }
  end

  test "ids that would traverse the path raise without a request" do
    assert_raises(Fizzy::Client::Error) { @client.card("../my", 579) }
    assert_raises(Fizzy::Client::Error) { @client.card("897362094", "579/x") }
    assert_raises(Fizzy::Client::Error) { @client.create_card("897362094", "../../x", title: "t") }

    assert_not_requested :any, %r{app\.fizzy\.do}
  end
end
