require "test_helper"

class Fizzy::ConnectionsControllerTest < ActionDispatch::IntegrationTest
  include FizzyTestHelper

  setup do
    sign_in :david
    grant_sudo_access
  end

  test "linking validates the token with GET /my/identity and stores the account" do
    stub = stub_fizzy_identity("fizzy_pat_pasted")

    post fizzy_connection_url, params: { access_token: "fizzy_pat_pasted" }

    assert_requested stub
    assert_redirected_to user_profile_path
    assert_equal "Fizzy connected as David (Smart Data).", flash[:notice]

    account = users(:david).reload.fizzy_connected_account
    assert_predicate account, :usable?
    assert_equal "897362094", account.fizzy_account_id
    assert_equal "Smart Data", account.fizzy_account_name
    assert_equal "03user1", account.fizzy_user_id
    assert_equal "David", account.fizzy_user_name
    assert_equal "fizzy_pat_pasted", account.access_token
  end

  test "linking with a blank token asks for one without a request" do
    post fizzy_connection_url, params: { access_token: "  " }

    assert_redirected_to user_profile_path
    assert_equal "Paste a token to connect Fizzy.", flash[:alert]
    assert_not_requested :any, %r{app\.fizzy\.do}
  end

  test "a rejected token is refused without storing" do
    stub_request(:get, "https://app.fizzy.do/my/identity.json").to_return(status: 401, body: {}.to_json)

    post fizzy_connection_url, params: { access_token: "bad-token" }

    assert_redirected_to user_profile_path
    assert_equal "Fizzy rejected that token. Check it and try again.", flash[:alert]
    assert_nil users(:david).reload.fizzy_connected_account
  end

  test "a token with no accounts is refused" do
    stub_fizzy_identity("fizzy_pat_pasted", payload: { "id" => "identity-1", "accounts" => [] })

    post fizzy_connection_url, params: { access_token: "fizzy_pat_pasted" }

    assert_equal "That token has no Fizzy account to use.", flash[:alert]
    assert_nil users(:david).reload.fizzy_connected_account
  end

  test "a transport failure asks to try again" do
    stub_request(:get, "https://app.fizzy.do/my/identity.json").to_timeout

    post fizzy_connection_url, params: { access_token: "fizzy_pat_pasted" }

    assert_equal "Could not reach Fizzy. Try again.", flash[:alert]
    assert_nil users(:david).reload.fizzy_connected_account
  end

  test "relinking replaces the stored token and clears the disconnect" do
    account = link_fizzy!(users(:david), token: "old-token")
    account.mark_disconnected!("Fizzy rejected the linked token (401)")
    stub_fizzy_identity("new-token")

    post fizzy_connection_url, params: { access_token: "new-token" }

    account = users(:david).reload.fizzy_connected_account
    assert_predicate account, :usable?
    assert_equal "new-token", account.access_token
  end

  test "disconnecting deletes the account" do
    link_fizzy!(users(:david))

    delete fizzy_connection_url

    assert_redirected_to user_profile_path
    assert_equal "Fizzy disconnected.", flash[:notice]
    assert_nil users(:david).reload.fizzy_connected_account
  end

  test "disconnecting deletes the member's card caches but keeps others" do
    link_fizzy!(users(:david))
    card = Fizzy::Card.for_reference(account_id: "897362094", number: 579)
    david_cache = Fizzy::CardCache.for_viewer(card: card, user: users(:david))
    jz_cache = Fizzy::CardCache.for_viewer(card: card, user: users(:jz))

    delete fizzy_connection_url

    assert_empty Fizzy::CardCache.where(id: david_cache.id)
    assert Fizzy::CardCache.exists?(jz_cache.id)
  end
end
