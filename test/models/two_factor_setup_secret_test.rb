require "test_helper"

class TwoFactorSetupSecretTest < ActiveSupport::TestCase
  setup do
    @session = sessions(:david_safari)
  end

  test "issue_for! rotates the secret on every call" do
    first = TwoFactorSetupSecret.issue_for!(@session)
    second = TwoFactorSetupSecret.issue_for!(@session)

    assert_not_equal first.secret, second.secret
    assert_equal 1, TwoFactorSetupSecret.where(session_id: @session.id).count
  end

  test "issue_for! scopes secrets to their session" do
    other_session = users(:jason).sessions.create!(user_agent: "Browser", ip_address: "1.2.3.4")

    first = TwoFactorSetupSecret.issue_for!(@session)
    second = TwoFactorSetupSecret.issue_for!(other_session)

    assert_not_equal first.secret, second.secret
    assert_equal first.secret, TwoFactorSetupSecret.valid_for(@session).secret
    assert_equal second.secret, TwoFactorSetupSecret.valid_for(other_session).secret
  end

  test "valid_for returns nil once expired" do
    record = TwoFactorSetupSecret.issue_for!(@session)
    record.update!(expires_at: 1.second.ago)

    assert_nil TwoFactorSetupSecret.valid_for(@session)
  end

  test "valid_for returns nil without a secret" do
    assert_nil TwoFactorSetupSecret.valid_for(@session)
  end

  test "secret is encrypted at rest" do
    record = TwoFactorSetupSecret.issue_for!(@session)
    stored = TwoFactorSetupSecret.connection.select_value(
      "SELECT secret FROM two_factor_setup_secrets WHERE id = #{record.id}")

    assert_not_equal record.secret, stored
    assert_equal record.secret, record.reload.secret
  end
end
