require "test_helper"

class Github::AppConnectionsControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    grant_sudo_access
    @env_before = [ ENV["GITHUB_APP_CLIENT_ID"], ENV["GITHUB_APP_CLIENT_SECRET"] ]
    ENV["GITHUB_APP_CLIENT_ID"] = "app-client-id"
    ENV["GITHUB_APP_CLIENT_SECRET"] = "app-client-secret"
  end

  teardown do
    ENV["GITHUB_APP_CLIENT_ID"], ENV["GITHUB_APP_CLIENT_SECRET"] = @env_before
  end

  test "connect redirects to the GitHub App authorize URL" do
    get github_app_connect_url

    assert_response :redirect
    assert_equal "github.com", URI.parse(response.location).host
  end

  test "connect answers 404 while the App is unconfigured" do
    ENV.delete("GITHUB_APP_CLIENT_ID")

    get github_app_connect_url

    assert_response :not_found
  end

  test "callback exchanges the code and stores an app token" do
    get github_app_connect_url
    state = Rails.application.message_verifier("github_app_oauth_state")
      .verified(CGI.parse(URI.parse(response.location).query)["state"].first)

    stub_request(:post, "https://github.com/login/oauth/access_token")
      .to_return(status: 200, body: {
        access_token: "app-token", refresh_token: "refresh-token", expires_in: 28_800
      }.to_json)
    stub_request(:get, "https://api.github.com/user")
      .to_return(status: 200, body: { login: "octocat" }.to_json)

    # The signed state round-trips through the session; rebuild a valid one.
    get github_app_callback_url, params: {
      code: "code",
      state: Rails.application.message_verifier("github_app_oauth_state").generate(state)
    }

    assert_redirected_to user_profile_path
    account = users(:david).reload.github_connected_account
    assert_predicate account, :usable?
    assert_predicate account, :app_token?
    assert_equal "octocat", account.github_login
    assert_equal "app-token", account.access_token
    assert_equal "refresh-token", account.refresh_token
    assert_in_delta 8.hours.from_now.to_i, account.token_expires_at.to_i, 60
  end

  test "callback with a stale state sends the member back to try again" do
    get github_app_callback_url, params: { code: "code", state: "bogus" }

    assert_redirected_to user_profile_path
    assert_equal "GitHub connection expired. Try again.", flash[:alert]
    assert_nil users(:david).reload.github_connected_account
  end

  test "callback when GitHub reports an error" do
    get github_app_connect_url
    session_state = session[:github_app_oauth_state]

    get github_app_callback_url, params: {
      error: "access_denied",
      state: Rails.application.message_verifier("github_app_oauth_state").generate(session_state)
    }

    assert_redirected_to user_profile_path
    assert_equal "GitHub connection was not approved.", flash[:alert]
  end

  test "callback answers 404 while the App is unconfigured" do
    ENV.delete("GITHUB_APP_CLIENT_SECRET")

    get github_app_callback_url, params: { code: "code", state: "x" }

    assert_response :not_found
  end

  test "disconnect revokes the app token remotely" do
    GithubConnectedAccount.create!(
      user: users(:david), github_login: "octocat", access_token: "app-token",
      token_source: "app", refresh_token: "refresh-token",
      token_expires_at: 1.hour.from_now
    )
    revoke = stub_request(:delete, "https://api.github.com/applications/app-client-id/grant")
      .to_return(status: 204)

    delete github_connection_url

    assert_requested revoke
    assert_nil users(:david).reload.github_connected_account
  end

  test "disconnect refreshes an expired token before revoking the grant" do
    GithubConnectedAccount.create!(
      user: users(:david), github_login: "octocat", access_token: "old-app-token",
      token_source: "app", refresh_token: "old-refresh",
      token_expires_at: 1.minute.ago
    )
    refresh = stub_request(:post, "https://github.com/login/oauth/access_token")
      .to_return(status: 200, body: {
        access_token: "fresh-token", refresh_token: "fresh-refresh", expires_in: 28_800
      }.to_json)
    grant = stub_request(:delete, "https://api.github.com/applications/app-client-id/grant")
      .with(body: hash_including("access_token" => "fresh-token"))
      .to_return(status: 204)
    stale = stub_request(:delete, "https://api.github.com/applications/app-client-id/grant")
      .with(body: hash_including("access_token" => "old-app-token"))

    delete github_connection_url

    assert_requested refresh
    assert_requested grant
    assert_not_requested stale
    assert_nil users(:david).reload.github_connected_account
  end

  test "disconnect proceeds when the refresh fails" do
    GithubConnectedAccount.create!(
      user: users(:david), github_login: "octocat", access_token: "old-app-token",
      token_source: "app", refresh_token: "old-refresh",
      token_expires_at: 1.minute.ago
    )
    stub_request(:post, "https://github.com/login/oauth/access_token").to_timeout
    grant = stub_request(:delete, "https://api.github.com/applications/app-client-id/grant")

    delete github_connection_url

    assert_redirected_to user_profile_path
    assert_not_requested grant
    assert_nil users(:david).reload.github_connected_account
  end

  test "disconnect proceeds when revocation fails" do
    GithubConnectedAccount.create!(
      user: users(:david), github_login: "octocat", access_token: "app-token",
      token_source: "app", refresh_token: "refresh-token",
      token_expires_at: 1.hour.from_now
    )
    stub_request(:delete, "https://api.github.com/applications/app-client-id/grant").to_timeout

    delete github_connection_url

    assert_redirected_to user_profile_path
    assert_nil users(:david).reload.github_connected_account
  end

  test "disconnect sends no revocation for a PAT" do
    GithubConnectedAccount.create!(
      user: users(:david), github_login: "octocat", access_token: "pat-token", token_source: "pat"
    )
    revoke = stub_request(:delete, %r{api\.github\.com/applications/})

    delete github_connection_url

    assert_not_requested revoke
    assert_nil users(:david).reload.github_connected_account
  end

  test "linking a PAT over an app connection resets the source" do
    GithubConnectedAccount.create!(
      user: users(:david), github_login: "octocat", access_token: "app-token",
      token_source: "app", refresh_token: "refresh-token",
      token_expires_at: 1.hour.from_now
    )
    stub_request(:get, "https://api.github.com/user")
      .to_return(status: 200, body: { login: "octocat" }.to_json)
    revoke = stub_request(:delete, "https://api.github.com/applications/app-client-id/token")
      .with(body: hash_including("access_token" => "app-token"))
      .to_return(status: 204)
    grant = stub_request(:delete, "https://api.github.com/applications/app-client-id/grant")

    post github_connection_url, params: { access_token: "github_pat_new" }

    account = users(:david).reload.github_connected_account
    assert_not account.app_token?
    assert_nil account.refresh_token
    assert_nil account.token_expires_at
    assert_requested revoke
    assert_not_requested grant
  end

  test "reconnecting through the app revokes only the previous app token" do
    GithubConnectedAccount.create!(
      user: users(:david), github_login: "octocat", access_token: "old-app-token",
      token_source: "app", refresh_token: "old-refresh",
      token_expires_at: 1.minute.ago
    )
    get github_app_connect_url
    state = Rails.application.message_verifier("github_app_oauth_state")
      .verified(CGI.parse(URI.parse(response.location).query)["state"].first)

    stub_request(:post, "https://github.com/login/oauth/access_token")
      .to_return(status: 200, body: {
        access_token: "app-token", refresh_token: "refresh-token", expires_in: 28_800
      }.to_json)
    stub_request(:get, "https://api.github.com/user")
      .to_return(status: 200, body: { login: "octocat" }.to_json)
    revoke = stub_request(:delete, "https://api.github.com/applications/app-client-id/token")
      .with(body: hash_including("access_token" => "old-app-token"))
      .to_return(status: 204)
    grant = stub_request(:delete, "https://api.github.com/applications/app-client-id/grant")

    get github_app_callback_url, params: {
      code: "code",
      state: Rails.application.message_verifier("github_app_oauth_state").generate(state)
    }

    assert_redirected_to user_profile_path
    assert_requested revoke
    assert_not_requested grant
    assert_equal "app-token", users(:david).reload.github_connected_account.access_token
  end
end
