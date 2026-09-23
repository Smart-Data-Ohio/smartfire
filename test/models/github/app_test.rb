require "test_helper"

class Github::AppTest < ActiveSupport::TestCase
  setup do
    @env_before = [ ENV["GITHUB_APP_CLIENT_ID"], ENV["GITHUB_APP_CLIENT_SECRET"] ]
    ENV["GITHUB_APP_CLIENT_ID"] = "app-client-id"
    ENV["GITHUB_APP_CLIENT_SECRET"] = "app-client-secret"
  end

  teardown do
    ENV["GITHUB_APP_CLIENT_ID"], ENV["GITHUB_APP_CLIENT_SECRET"] = @env_before
  end

  test "unconfigured without both credentials" do
    ENV.delete("GITHUB_APP_CLIENT_ID")
    assert_not Github::App.configured?
  end

  test "authorize_url points at github.com with the client id" do
    url = URI.parse(Github::App.authorize_url(redirect_uri: "https://app.test/callback", state: "state"))
    assert_equal "github.com", url.host
    assert_equal "app-client-id", URI.decode_www_form(url.query).to_h["client_id"]
  end

  test "exchange_code returns the token response" do
    stub_request(:post, "https://github.com/login/oauth/access_token")
      .to_return(status: 200, body: {
        access_token: "app-token", refresh_token: "refresh-token", expires_in: 28_800
      }.to_json)

    tokens = Github::App.exchange_code(code: "code", redirect_uri: "https://app.test/callback")
    assert_equal "app-token", tokens["access_token"]
    assert_equal "refresh-token", tokens["refresh_token"]
  end

  test "exchange_code raises Unauthorized on invalid_grant" do
    stub_request(:post, "https://github.com/login/oauth/access_token")
      .to_return(status: 200, body: { error: "invalid_grant" }.to_json)

    assert_raises(Github::App::Unauthorized) do
      Github::App.exchange_code(code: "bad", redirect_uri: "https://app.test/callback")
    end
  end

  test "exchange_code raises Error on transport failure" do
    stub_request(:post, "https://github.com/login/oauth/access_token").to_timeout

    error = assert_raises(Github::App::Error) do
      Github::App.exchange_code(code: "code", redirect_uri: "https://app.test/callback")
    end
    assert_not_includes error.message, "app-client-secret"
  end

  test "refresh_access_token posts the refresh grant" do
    stub = stub_request(:post, "https://github.com/login/oauth/access_token")
      .with(body: hash_including("grant_type" => "refresh_token", "refresh_token" => "old-refresh"))
      .to_return(status: 200, body: {
        access_token: "new-token", refresh_token: "new-refresh", expires_in: 28_800
      }.to_json)

    tokens = Github::App.refresh_access_token(refresh_token: "old-refresh")
    assert_requested stub
    assert_equal "new-token", tokens["access_token"]
  end

  test "revoke_token deletes one token and never raises" do
    stub_request(:delete, "https://api.github.com/applications/app-client-id/token")
      .to_return(status: 204)
    assert Github::App.revoke_token("app-token")

    WebMock.reset!
    stub_request(:delete, "https://api.github.com/applications/app-client-id/token")
      .to_return(status: 403)
    assert_not Github::App.revoke_token("app-token")

    WebMock.reset!
    stub_request(:delete, "https://api.github.com/applications/app-client-id/token").to_timeout
    assert_not Github::App.revoke_token("app-token")
  end

  test "revoke_grant deletes the whole authorization" do
    grant = stub_request(:delete, "https://api.github.com/applications/app-client-id/grant")
      .to_return(status: 204)

    assert Github::App.revoke_grant("app-token")
    assert_requested grant
  end

  test "revoke_grant treats an unknown token as failure, revoke_token as gone" do
    stub_request(:delete, "https://api.github.com/applications/app-client-id/grant")
      .to_return(status: 404)
    stub_request(:delete, "https://api.github.com/applications/app-client-id/token")
      .to_return(status: 404)

    assert_not Github::App.revoke_grant("app-token")
    assert Github::App.revoke_token("old-token")
  end
end
