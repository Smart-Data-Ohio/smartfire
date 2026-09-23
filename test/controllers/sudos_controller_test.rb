require "test_helper"

class SudosControllerTest < ActionDispatch::IntegrationTest
  include GoogleCalendarTestHelper
  include GoogleSignInTestHelper

  test "the prompt shows the password form for password users" do
    sign_in users(:david)

    get new_sudo_url

    assert_response :success
    assert_select "form[action=?]", sudo_url do
      assert_select "input[name=password][type=password]"
    end
    assert_select "form[action=?]", sudo_google_url, count: 0
  end

  test "the prompt shows Google confirmation for Google-only users" do
    sign_in_google_only_user

    get new_sudo_url

    assert_response :success
    assert_select "input[name=password]", count: 0
    assert_select "form[action=?]", sudo_google_path, count: 1
  end

  test "confirming with the password verifies and audit-logs" do
    sign_in users(:david)

    assert_difference -> { AuditLog.where(action: "sudo.confirm.success").count }, +1 do
      post sudo_url, params: { password: "secret123456" }
    end

    assert_redirected_to root_url
  end

  test "confirming with the wrong password fails and stays gated" do
    sign_in users(:david)

    assert_difference -> { AuditLog.where(action: "sudo.confirm.failure").count }, +1 do
      post sudo_url, params: { password: "wrong" }
    end

    assert_response :unauthorized

    post account_join_code_url
    assert_redirected_to new_sudo_url
  end

  test "an unknown verifier is rejected as unavailable" do
    sign_in users(:david)

    post sudo_url, params: { verifier: "totp", totp_code: "123456" }

    assert_response :unprocessable_entity
    assert_match "not available", response.body
  end

  test "totp is an unsupported hook until two-factor lands" do
    assert_equal :unsupported, SudoMode.verify_with(:totp, users(:david), { totp_code: "123456" })
    assert_not SudoMode.verifier_available?(:totp, users(:david))
  end

  test "register_verifier adds a verifier for two-factor to fill in" do
    SudoMode.register_verifier(:totp)

    assert_includes SudoMode.extra_verifiers, :totp
  ensure
    SudoMode.extra_verifiers.delete(:totp)
  end

  test "a gated POST redirects to the prompt, then the replay form continues the action" do
    sign_in users(:david)
    previous_code = Current.account.join_code

    post account_join_code_url
    assert_redirected_to new_sudo_url

    get new_sudo_url
    assert_response :success

    post sudo_url, params: { password: "secret123456" }
    assert_response :success
    assert_select "form[action=?][method=post]", account_join_code_path, count: 1

    # The replay form resubmits the stashed request with a fresh token.
    post account_join_code_url
    assert_redirected_to edit_account_url
    assert_not_equal previous_code, Current.account.reload.join_code
  end

  test "a gated GET continues automatically after confirmation" do
    sign_in users(:david)

    get account_audit_log_url(format: :csv)
    assert_redirected_to new_sudo_url

    post sudo_url, params: { password: "secret123456" }
    assert_redirected_to account_audit_log_url(format: :csv)

    follow_redirect!
    assert_response :success
    assert_equal "text/csv", response.media_type
  end

  test "browsing the audit log needs no confirmation" do
    sign_in users(:david)

    get account_audit_log_url

    assert_response :success
  end

  test "a fresh confirmation lasts fifteen minutes" do
    sign_in users(:david)
    grant_sudo_access

    travel 14.minutes do
      post account_join_code_url
      assert_redirected_to edit_account_url
    end
  end

  test "a stale confirmation prompts again" do
    sign_in users(:david)
    grant_sudo_access

    travel 16.minutes do
      post account_join_code_url
      assert_redirected_to new_sudo_url
    end
  end

  test "signing in again starts unverified" do
    sign_in users(:david)
    grant_sudo_access

    post session_url, params: { email_address: "david@37signals.com", password: "secret123456" }

    post account_join_code_url
    assert_redirected_to new_sudo_url
  end

  test "requests carrying secrets are never replayed" do
    sign_in users(:kevin)

    post fizzy_connection_url, params: { access_token: "pasted-token" }
    assert_redirected_to new_sudo_url

    post sudo_url, params: { password: "secret123456" }
    # No replay form: back to the originating page instead.
    assert_response :redirect
    assert_select "form[action=?]", fizzy_connection_path, count: 0
  end

  test "the replay form rebuilds nested params" do
    sign_in users(:david)

    patch account_user_url(users(:kevin)), params: { user: { role: "administrator" } }
    assert_redirected_to new_sudo_url

    post sudo_url, params: { password: "secret123456" }
    assert_response :success
    assert_select "form[action=?]", account_user_path(users(:kevin)) do
      assert_select "input[name=?][value=administrator]", "user[role]"
    end
  end

  test "resuming after confirmation never leaves the app" do
    sign_in users(:kevin)

    post fizzy_connection_url, params: { access_token: "pasted-token" },
      headers: { "Referer" => "https://evil.example.test/phish" }
    assert_redirected_to new_sudo_url

    post sudo_url, params: { password: "secret123456" }
    assert_redirected_to root_url
  end

  test "non-replayable requests resume on the originating page" do
    sign_in users(:kevin)

    post fizzy_connection_url, params: { access_token: "pasted-token" },
      headers: { "Referer" => user_profile_url }
    assert_redirected_to new_sudo_url

    post sudo_url, params: { password: "secret123456" }
    assert_redirected_to user_profile_path
  end

  test "editing a bot without touching the webhook needs no confirmation" do
    sign_in users(:david)
    bot = users(:bender)

    patch account_bot_url(bot), params: { user: { name: "Bender Prime" } }

    assert_redirected_to account_bots_url
    assert_equal "Bender Prime", bot.reload.name
  end

  test "changing a bot webhook needs confirmation" do
    sign_in users(:david)

    patch account_bot_url(users(:bender)), params: { user: { webhook_url: "https://hooks.example.test/room" } }

    assert_redirected_to new_sudo_url
  end

  test "submitting a bot webhook unchanged needs no confirmation" do
    sign_in users(:david)
    bot = users(:bender)

    patch account_bot_url(bot), params: { user: { name: "Bender Prime", webhook_url: bot.webhook_url } }

    assert_redirected_to account_bots_url
  end

  test "Google re-auth confirms a Google-only user and continues" do
    user = sign_in_google_only_user

    delete fizzy_connection_url
    assert_redirected_to new_sudo_url

    post sudo_google_url
    assert_response :redirect
    query = Rack::Utils.parse_query(URI(response.location).query)

    stub_google_jwks
    stub_sign_in_code_exchange(id_token: sign_in_id_token(sub: "google-sub-alice", nonce: query["nonce"]))

    assert_difference -> { AuditLog.where(action: "sudo.confirm.success").count }, +1 do
      get session_google_callback_path, params: { state: query["state"], code: "auth-code" }
    end

    # The stashed DELETE replays through the continue form.
    assert_response :success
    assert_select "form[action=?]", fizzy_connection_path

    delete fizzy_connection_url
    assert_redirected_to user_profile_path

    # Still the same member: confirmation signs nobody in or out.
    get user_profile_url
    assert_response :success
    assert_select "input[value='alice@smartdata.net']"
  end

  test "Google re-auth with a different Google account is rejected" do
    sign_in_google_only_user

    delete fizzy_connection_url
    assert_redirected_to new_sudo_url

    post sudo_google_url
    query = Rack::Utils.parse_query(URI(response.location).query)

    stub_google_jwks
    stub_sign_in_code_exchange(id_token: sign_in_id_token(sub: "google-sub-mallory", email: "mallory@smartdata.net", nonce: query["nonce"]))

    assert_difference -> { AuditLog.where(action: "sudo.confirm.failure").count }, +1 do
      get session_google_callback_path, params: { state: query["state"], code: "auth-code" }
    end

    assert_redirected_to new_sudo_url

    delete fizzy_connection_url
    assert_redirected_to new_sudo_url
  end

  test "Google re-auth forces a fresh Google login" do
    sign_in_google_only_user

    post sudo_google_url

    assert_response :redirect
    query = Rack::Utils.parse_query(URI(response.location).query)
    assert_equal "login", query["prompt"]
    assert_equal "0", query["max_age"]
  end

  test "Google re-auth with a stale Google login is rejected" do
    sign_in_google_only_user

    delete fizzy_connection_url
    assert_redirected_to new_sudo_url

    post sudo_google_url
    query = Rack::Utils.parse_query(URI(response.location).query)

    stub_google_jwks
    stub_sign_in_code_exchange(id_token: sign_in_id_token(sub: "google-sub-alice", nonce: query["nonce"],
      auth_time: 6.minutes.ago.to_i))

    get session_google_callback_path, params: { state: query["state"], code: "auth-code" }

    assert_redirected_to new_sudo_url
    assert_match "confirmation failed", flash[:alert].downcase

    delete fizzy_connection_url
    assert_redirected_to new_sudo_url
  end

  test "Google re-auth without an auth_time is rejected" do
    sign_in_google_only_user

    delete fizzy_connection_url
    assert_redirected_to new_sudo_url

    post sudo_google_url
    query = Rack::Utils.parse_query(URI(response.location).query)

    stub_google_jwks
    stub_sign_in_code_exchange(id_token: sign_in_id_token(sub: "google-sub-alice", nonce: query["nonce"],
      auth_time: nil))

    get session_google_callback_path, params: { state: query["state"], code: "auth-code" }

    assert_redirected_to new_sudo_url

    delete fizzy_connection_url
    assert_redirected_to new_sudo_url
  end

  test "Google confirmation is unavailable without a linked identity" do
    sign_in users(:david)

    post sudo_google_url

    assert_redirected_to new_sudo_url
  end

  test "confirmation attempts are rate limited" do
    sign_in users(:david)

    10.times { post sudo_url, params: { password: "wrong" } }
    post sudo_url, params: { password: "wrong" }

    assert_response :too_many_requests
  end

  test "the confirmation limit lives in the shared rate-limit store, not per-process memory" do
    sign_in users(:david)

    10.times { post sudo_url, params: { password: "wrong" } }
    # The count belongs to the shared store: clearing it resets the
    # limit. A per-process store would still reject this attempt.
    ActionController::Base.cache_store.clear
    post sudo_url, params: { password: "wrong" }

    assert_response :unauthorized
  end

  private
    # Provisions Alice through the stubbed Google sign-in flow: no
    # password, linked identity for google-sub-alice, signed in.
    def sign_in_google_only_user
      state = start_google_sign_in
      complete_google_sign_in(state:, email: "alice@smartdata.net", hd: "smartdata.net", sub: "google-sub-alice")
      assert_response :redirect

      User.find_by!(email_address: "alice@smartdata.net")
    end
end
