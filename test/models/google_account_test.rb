require "test_helper"

class GoogleAccountTest < ActiveSupport::TestCase
  include GoogleCalendarTestHelper

  test "one account per user" do
    connect_google!(users(:david))

    duplicate = GoogleAccount.new(user: users(:david), email: "other@gmail.test")

    assert_not duplicate.valid?
    assert_equal [ "has already been taken" ], duplicate.errors[:user_id]
  end

  test "tokens round-trip encrypted at rest" do
    account = connect_google!(users(:david))

    raw = GoogleAccount.connection.select_value(
      "SELECT refresh_token FROM google_accounts WHERE id = #{account.id}"
    )

    assert_not_equal "refresh-token-#{users(:david).id}", raw
    assert_equal "refresh-token-#{users(:david).id}", account.reload.refresh_token
  end

  test "drive? reflects the stored scopes" do
    account = connect_google!(users(:david))

    assert_not_predicate account, :drive?

    account.update!(scopes: "openid email https://www.googleapis.com/auth/calendar.events")

    assert_not_predicate account, :drive?

    account.update!(scopes: "openid email https://www.googleapis.com/auth/calendar.events https://www.googleapis.com/auth/drive.metadata.readonly")

    assert_predicate account, :drive?
  end

  test "connected, usable, and expiry predicates" do
    account = connect_google!(users(:david))

    assert_predicate account, :connected?
    assert_predicate account, :usable?
    assert_not account.access_token_expired?

    account.update!(access_token_expires_at: 1.minute.ago)

    assert account.access_token_expired?

    account.mark_disconnected!("Google rejected the connection")

    assert_not_predicate account, :connected?
    assert_not_predicate account, :usable?
  end

  test "calendar? treats blank scopes as granted and requires calendar.events otherwise" do
    account = connect_google!(users(:david))

    assert_predicate account, :calendar?

    account.update!(scopes: "openid email")

    assert_not_predicate account, :calendar?

    account.update!(scopes: CALENDAR_SCOPES)

    assert_predicate account, :calendar?
  end

  test "an unreadable token reads as unusable and marks the account disconnected" do
    account = connect_google!(users(:david))
    corrupt_google_token!(account)

    assert_not_predicate account, :usable?
    assert_equal GoogleAccount::UNREADABLE_TOKEN_REASON, account.reload.disconnected_reason
    assert_not_predicate account, :connected?
  end

  test "cleanup_snapshot returns the tokens, or nil when unreadable" do
    account = connect_google!(users(:david))

    snapshot = account.cleanup_snapshot

    assert_equal "refresh-token-#{users(:david).id}", snapshot[:refresh_token]
    assert_equal "access-token-#{users(:david).id}", snapshot[:access_token]
    assert_not_nil snapshot[:access_token_expires_at]

    corrupt_google_token!(account)

    assert_nil account.cleanup_snapshot
  end
end
