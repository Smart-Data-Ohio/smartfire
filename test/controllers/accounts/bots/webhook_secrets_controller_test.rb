require "test_helper"

class Accounts::Bots::WebhookSecretsControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    grant_sudo_access
    @bot = users(:bender)
    @agent = agents(:bender_agent)
  end

  test "admin resets the agent secret" do
    old_secret = @agent.ensure_webhook_signing_secret!

    post account_bot_webhook_secret_url(@bot)

    assert_redirected_to edit_account_bot_url(@bot)
    assert_not_equal old_secret, @agent.reload.webhook_signing_secret
    assert @agent.reload.webhook_signing_secret.present?
  end

  test "agent owner without admin rights resets the agent secret" do
    @agent.update!(owner: users(:kevin))
    sign_in users(:kevin)
    grant_sudo_access
    old_secret = @agent.ensure_webhook_signing_secret!

    post account_bot_webhook_secret_url(@bot)

    assert_redirected_to edit_account_bot_url(@bot)
    assert_not_equal old_secret, @agent.reload.webhook_signing_secret
  end

  test "non-owner cannot reset the secret" do
    sign_in users(:kevin)
    old_secret = @agent.ensure_webhook_signing_secret!

    post account_bot_webhook_secret_url(@bot)

    assert_response :forbidden
    assert_equal old_secret, @agent.reload.webhook_signing_secret
  end

  test "legacy bot secret is generated on the webhook" do
    legacy = User.create_bot!(name: "Legacy Secret", webhook_url: "https://example.test/legacy-secret")
    assert_nil legacy.webhook.signing_secret

    post account_bot_webhook_secret_url(legacy)

    assert_redirected_to edit_account_bot_url(legacy)
    assert legacy.webhook.reload.signing_secret.present?
    assert_nil legacy.reload.agent
  end

  test "legacy bot without a webhook URL redirects with an alert" do
    legacy = User.create_bot!(name: "Legacy Nosecret")

    post account_bot_webhook_secret_url(legacy)

    assert_redirected_to edit_account_bot_url(legacy)
    assert_nil legacy.reload.webhook
  end

  test "non-owner cannot reset a legacy secret" do
    legacy = User.create_bot!(name: "Legacy Guarded", webhook_url: "https://example.test/legacy-guarded")
    sign_in users(:kevin)

    post account_bot_webhook_secret_url(legacy)

    assert_response :forbidden
    assert_nil legacy.webhook.reload.signing_secret
  end

  test "bot edit page shows the signing secret" do
    secret = @agent.ensure_webhook_signing_secret!

    get edit_account_bot_url(@bot)

    assert_response :success
    assert_match "Webhook signing secret", response.body
    assert_match secret, response.body
  end
end
