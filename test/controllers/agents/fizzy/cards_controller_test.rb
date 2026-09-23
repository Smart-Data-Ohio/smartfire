require "test_helper"

class Agents::Fizzy::CardsControllerTest < ActionDispatch::IntegrationTest
  include FizzyTestHelper

  setup do
    @agent = agents(:bender_agent)
    @secret = "bender-test-secret-1234"
  end

  test "search returns matching cards" do
    link_owner_fizzy!
    grant_fizzy!
    stub = stub_request(:get, "https://app.fizzy.do/897362094/search.json?q=billing")
      .to_return(status: 200, body: [ fizzy_card_payload ].to_json)

    get "/agents/fizzy/cards/search?q=billing", headers: bearer_headers

    assert_response :success
    assert_requested stub
    assert_equal [ 579 ], response.parsed_body.map { |card| card["number"] }
  end

  test "search without a query is 422" do
    link_owner_fizzy!
    grant_fizzy!

    get "/agents/fizzy/cards/search", headers: bearer_headers

    assert_response :unprocessable_entity
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "show returns one card with steps" do
    link_owner_fizzy!
    grant_fizzy!
    stub = stub_fizzy_card(579, token: "owner-token-abc")

    get "/agents/fizzy/cards/897362094/579", headers: bearer_headers

    assert_response :success
    assert_requested stub
    assert_equal "Fix the billing bug", response.parsed_body["title"]
    assert_equal 2, response.parsed_body["steps"].size
  end

  test "show for an inaccessible card is 404" do
    link_owner_fizzy!
    grant_fizzy!
    stub_request(:get, "https://app.fizzy.do/897362094/cards/404.json")
      .to_return(status: 404, body: {}.to_json)

    get "/agents/fizzy/cards/897362094/404", headers: bearer_headers

    assert_response :not_found
  end

  test "show with a bad number is 404 without a request" do
    link_owner_fizzy!
    grant_fizzy!

    get "/agents/fizzy/cards/897362094/abc", headers: bearer_headers

    assert_response :not_found
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "reads without the fizzy capability are 403" do
    link_owner_fizzy!

    get "/agents/fizzy/cards/search?q=x", headers: bearer_headers
    assert_response :forbidden

    get "/agents/fizzy/cards/897362094/579", headers: bearer_headers
    assert_response :forbidden

    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "reads without an owner account are 422" do
    grant_fizzy!

    get "/agents/fizzy/cards/search?q=x", headers: bearer_headers

    assert_response :unprocessable_entity
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  private
    def bearer_headers
      { "Authorization" => "Bearer #{@secret}", "Content-Type" => "application/json" }
    end

    def grant_fizzy!
      AgentGrant.create!(agent: @agent, room: nil, granted_by: users(:david), capability: "fizzy")
    end

    def link_owner_fizzy!(token: "owner-token-abc")
      link_fizzy!(users(:david), token: token)
    end
end
