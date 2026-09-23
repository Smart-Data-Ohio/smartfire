require "test_helper"

class Fizzy::AgentCardActionTest < ActiveSupport::TestCase
  include FizzyTestHelper

  test "comment requires a card number and body" do
    action = Fizzy::AgentCardAction.new(account_id: "897362094", kind: "comment", number: 579, body: "Nice")

    assert_predicate action, :valid?
    assert_equal "fizzy.comment", action.action_name
    assert_equal "Comment on Fizzy card #579 (account 897362094): Nice", action.summary
  end

  test "create requires a board and title" do
    action = Fizzy::AgentCardAction.new(account_id: "897362094", kind: "create", board_id: "03board1", title: "Ship it")

    assert_predicate action, :valid?
    assert_equal "fizzy.create", action.action_name
    assert_includes action.summary, "Create Fizzy card"
    assert_includes action.summary, "Ship it"
  end

  test "move requires a card and column" do
    action = Fizzy::AgentCardAction.new(account_id: "897362094", kind: "move", number: 579, column_id: "03column1")

    assert_predicate action, :valid?
    assert_equal "fizzy.move", action.action_name
  end

  test "close and reopen require a card" do
    assert_predicate Fizzy::AgentCardAction.new(account_id: "897362094", kind: "close", number: 579), :valid?
    assert_predicate Fizzy::AgentCardAction.new(account_id: "897362094", kind: "reopen", number: 579), :valid?
  end

  test "unknown kinds and missing fields are invalid" do
    assert_not Fizzy::AgentCardAction.new(account_id: "897362094", kind: "explode", number: 579).valid?
    assert_not Fizzy::AgentCardAction.new(account_id: "897362094", kind: "comment", number: 579).valid?
    assert_not Fizzy::AgentCardAction.new(account_id: "897362094", kind: "comment", body: "x").valid?
    assert_not Fizzy::AgentCardAction.new(account_id: "897362094", kind: "create", title: "x").valid?
    assert_not Fizzy::AgentCardAction.new(account_id: "897362094", kind: "move", number: 579).valid?
    assert_not Fizzy::AgentCardAction.new(kind: "close", number: 579).valid?
  end

  test "overlong text is invalid" do
    assert_not Fizzy::AgentCardAction.new(account_id: "897362094", kind: "comment", number: 579, body: "x" * 3501).valid?
    assert_not Fizzy::AgentCardAction.new(account_id: "897362094", kind: "create", board_id: "b", title: "x" * 501).valid?
  end

  test "path-traversing ids are invalid" do
    assert_not Fizzy::AgentCardAction.new(account_id: "../my", kind: "close", number: 579).valid?
    assert_not Fizzy::AgentCardAction.new(account_id: "897362094", kind: "create", board_id: "a/b", title: "t").valid?
  end

  test "payload round-trips through from_payload" do
    action = Fizzy::AgentCardAction.new(account_id: "897362094", kind: "comment", number: 579, body: "Nice")
    rebuilt = Fizzy::AgentCardAction.from_payload(payload: JSON.parse(action.payload_json))

    assert_predicate rebuilt, :valid?
    assert_equal action.action_name, rebuilt.action_name
    assert_equal action.summary, rebuilt.summary
  end

  test "perform dispatches each kind to the client" do
    client = Fizzy::Client.new(token: "owner-token")
    comment = stub_request(:post, "https://app.fizzy.do/897362094/cards/579/comments.json")
      .to_return(status: 201, body: fizzy_comment_payload.to_json)
    create = stub_request(:post, "https://app.fizzy.do/897362094/boards/03board1/cards.json")
      .to_return(status: 201, body: fizzy_card_payload(number: 580).to_json)
    move = stub_request(:post, "https://app.fizzy.do/897362094/cards/579/triage.json").to_return(status: 204)
    close = stub_request(:post, "https://app.fizzy.do/897362094/cards/579/closure.json").to_return(status: 204)
    reopen = stub_request(:delete, "https://app.fizzy.do/897362094/cards/579/closure.json").to_return(status: 204)

    assert_equal "https://app.fizzy.do/897362094/cards/579/comments/03comment1",
      Fizzy::AgentCardAction.new(account_id: "897362094", kind: "comment", number: 579, body: "Nice").perform(client)["url"]
    assert_equal 580,
      Fizzy::AgentCardAction.new(account_id: "897362094", kind: "create", board_id: "03board1", title: "T").perform(client)["number"]
    assert Fizzy::AgentCardAction.new(account_id: "897362094", kind: "move", number: 579, column_id: "03column1").perform(client)
    assert Fizzy::AgentCardAction.new(account_id: "897362094", kind: "close", number: 579).perform(client)
    assert Fizzy::AgentCardAction.new(account_id: "897362094", kind: "reopen", number: 579).perform(client)

    assert_requested comment
    assert_requested create
    assert_requested move
    assert_requested close
    assert_requested reopen
  end
end
