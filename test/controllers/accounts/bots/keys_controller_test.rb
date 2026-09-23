require "test_helper"

class Accounts::Bots::KeysControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    grant_sudo_access
  end

  test "update issues a new key, shows it once, and retires the old one" do
    old_key = bot_key_for(users(:bender))

    assert_changes -> { users(:bender).reload.bot_token_digest } do
      put account_bot_key_url(users(:bender))
    end

    assert_response :success
    assert_equal "no-store", response.headers["Cache-Control"]
    new_key = css_select("input[aria-label='Bot key']").first["value"]
    assert_equal users(:bender), User.authenticate_bot(new_key)
    assert_nil User.authenticate_bot(old_key)

    get account_bots_url
    assert_not_includes response.body, new_key
  end

  test "members cannot reset a bot key" do
    sign_in :kevin

    assert_no_changes -> { users(:bender).reload.bot_token_digest } do
      put account_bot_key_url(users(:bender))
    end
    assert_response :forbidden
  end
end
