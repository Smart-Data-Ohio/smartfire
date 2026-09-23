require "test_helper"

class GithubConnectedAccountTest < ActiveSupport::TestCase
  test "one account per user" do
    connect_github!(users(:david))

    duplicate = GithubConnectedAccount.new(user: users(:david), github_login: "other", access_token: "x")

    assert_not duplicate.valid?
    assert_equal [ "has already been taken" ], duplicate.errors[:user_id]
  end

  test "token is encrypted at rest" do
    account = connect_github!(users(:david))

    raw = GithubConnectedAccount.connection.select_value(
      "SELECT access_token FROM github_connected_accounts WHERE id = #{account.id}"
    )

    assert_not_equal "github-token-#{users(:david).id}", raw
    assert_equal "github-token-#{users(:david).id}", account.reload.access_token
  end

  test "an undecryptable token is unusable, not fatal" do
    account = connect_github!(users(:david))
    GithubConnectedAccount.connection.execute(
      "UPDATE github_connected_accounts SET access_token = 'bogus-ciphertext' WHERE id = #{account.id}"
    )

    assert_nothing_raised do
      assert_not account.reload.usable?
    end
    assert_not_predicate account, :connected?
    assert_equal "The stored token could not be read; link it again", account.disconnected_reason
    assert_not account.usable?
  end

  test "connected, usable, and disconnect reason" do
    account = connect_github!(users(:david))

    assert_predicate account, :connected?
    assert_predicate account, :usable?

    account.mark_disconnected!("GitHub rejected the token (401)")

    assert_not_predicate account, :connected?
    assert_not_predicate account, :usable?
    assert_equal "GitHub rejected the token (401)", account.disconnected_reason
  end

  test "a fresh PAT is used as-is" do
    account = connect_github!(users(:david))

    assert_not account.app_token?
    assert_equal "github-token-#{users(:david).id}", account.access_token_for_use
  end

  test "refresh token is encrypted at rest" do
    account = connect_github!(users(:david), token_source: "app",
      refresh_token: "refresh-secret", token_expires_at: 1.hour.from_now)

    raw = GithubConnectedAccount.connection.select_value(
      "SELECT refresh_token FROM github_connected_accounts WHERE id = #{account.id}"
    )

    assert_not_equal "refresh-secret", raw
    assert_equal "refresh-secret", account.reload.refresh_token
  end

  test "an expired app token refreshes in place" do
    account = connect_github!(users(:david), token_source: "app",
      refresh_token: "old-refresh", token_expires_at: 1.minute.ago)
    stub_request(:post, "https://github.com/login/oauth/access_token")
      .to_return(status: 200, body: {
        access_token: "new-token", refresh_token: "new-refresh", expires_in: 28_800
      }.to_json)

    assert_equal "new-token", account.access_token_for_use

    account.reload
    assert_equal "new-token", account.access_token
    assert_equal "new-refresh", account.refresh_token
    assert_predicate account, :connected?
  end

  test "a rejected refresh disconnects the account" do
    account = connect_github!(users(:david), token_source: "app",
      refresh_token: "dead-refresh", token_expires_at: 1.minute.ago)
    stub_request(:post, "https://github.com/login/oauth/access_token")
      .to_return(status: 200, body: { error: "invalid_grant" }.to_json)

    assert_nil account.access_token_for_use
    assert_not_predicate account.reload, :connected?
  end

  test "a rejected refresh keeps a token another process already rotated" do
    account = connect_github!(users(:david), token_source: "app",
      refresh_token: "old-refresh", token_expires_at: 1.minute.ago)
    stub_request(:post, "https://github.com/login/oauth/access_token")
      .to_return(status: 200, body: { error: "invalid_grant" }.to_json)

    # Another process refreshes first with the same grant; this instance
    # still holds the stale, rotated-out refresh token.
    GithubConnectedAccount.find(account.id).update!(
      access_token: "rotated-token", refresh_token: "rotated-refresh",
      token_expires_at: 1.hour.from_now)

    assert_equal "rotated-token", account.access_token_for_use
    assert_predicate account.reload, :connected?
  end

  test "a failed refresh transport records last_error and keeps the old token" do
    account = connect_github!(users(:david), token_source: "app",
      refresh_token: "old-refresh", token_expires_at: 1.minute.ago)
    stub_request(:post, "https://github.com/login/oauth/access_token").to_timeout

    assert_nil account.access_token_for_use
    assert_predicate account.reload, :connected?
    assert account.last_error.present?
  end

  test "agent identity prefers the owner's usable app token" do
    agent = agents(:bender_agent)
    GithubConnectedAccount.create!(user: agent.user, github_login: "bender-machine", access_token: "agent-pat")
    owner_account = GithubConnectedAccount.create!(user: agent.owner, github_login: "owner-login",
      access_token: "owner-app-token", token_source: "app",
      refresh_token: "owner-refresh", token_expires_at: 1.hour.from_now)

    assert_equal owner_account, Github::AgentIdentity.resolve(agent)
  end

  test "agent identity falls back to the agent account without an owner app token" do
    agent = agents(:bender_agent)
    agent_account = GithubConnectedAccount.create!(user: agent.user, github_login: "bender-machine", access_token: "agent-pat")
    GithubConnectedAccount.create!(user: agent.owner, github_login: "owner-login", access_token: "owner-pat")

    assert_equal agent_account, Github::AgentIdentity.resolve(agent)
  end

  test "agent identity falls back when the owner app token is disconnected" do
    agent = agents(:bender_agent)
    agent_account = GithubConnectedAccount.create!(user: agent.user, github_login: "bender-machine", access_token: "agent-pat")
    GithubConnectedAccount.create!(user: agent.owner, github_login: "owner-login",
      access_token: "owner-app-token", token_source: "app",
      refresh_token: "owner-refresh", token_expires_at: 1.hour.from_now,
      disconnected_reason: "GitHub rejected the linked token (401)")

    assert_equal agent_account, Github::AgentIdentity.resolve(agent)
  end

  private
    def connect_github!(user, **attributes)
      GithubConnectedAccount.create!(
        user:,
        github_login: user.name.parameterize,
        access_token: "github-token-#{user.id}",
        **attributes
      )
    end
end
