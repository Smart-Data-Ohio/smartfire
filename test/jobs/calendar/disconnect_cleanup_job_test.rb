require "test_helper"

class Calendar::DisconnectCleanupJobTest < ActiveSupport::TestCase
  include GoogleCalendarTestHelper

  setup do
    @david = users(:david)
  end

  test "deletes every remote copy then revokes the grant" do
    account = connect_google!(@david)
    snapshot = account.cleanup_snapshot
    first_delete = stub_google_event_delete("first-id")
    second_delete = stub_google_event_delete("second-id")
    revoke = stub_google_revoke
    account.destroy!

    Calendar::DisconnectCleanupJob.perform_now([ "first-id", "second-id" ], snapshot)

    assert_requested first_delete, headers: { "Authorization" => "Bearer #{snapshot[:access_token]}" }
    assert_requested second_delete
    assert_requested revoke, body: hash_including({ "token" => snapshot[:refresh_token] })
  end

  test "refreshes an expired access token from the snapshot" do
    account = connect_google!(@david)
    account.update!(access_token_expires_at: 1.hour.ago)
    snapshot = account.cleanup_snapshot
    refreshed_token = "second-access-token"
    stub_google_token_refresh(access_token: refreshed_token)
    delete_stub = stub_google_event_delete("orphan-id")
    revoke = stub_google_revoke
    account.destroy!

    Calendar::DisconnectCleanupJob.perform_now([ "orphan-id" ], snapshot)

    assert_requested :post, GOOGLE_TOKEN_URL, body: hash_including({ "grant_type" => "refresh_token" })
    assert_requested delete_stub, headers: { "Authorization" => "Bearer #{refreshed_token}" }
    assert_requested revoke
  end

  test "an invalid_grant skips deletes but still revokes" do
    account = connect_google!(@david)
    account.update!(access_token_expires_at: 1.hour.ago)
    snapshot = account.cleanup_snapshot
    stub_google_token_invalid_grant
    revoke = stub_google_revoke
    account.destroy!

    Calendar::DisconnectCleanupJob.perform_now([ "orphan-id" ], snapshot)

    assert_not_requested :delete, "#{GOOGLE_EVENTS_URL}/orphan-id"
    assert_requested revoke
  end

  test "Google failures are best effort and never raise" do
    account = connect_google!(@david)
    snapshot = account.cleanup_snapshot
    delete_stub = stub_google_event_delete("orphan-id", status: 500)
    revoke = stub_google_revoke(status: 500)
    account.destroy!

    Calendar::DisconnectCleanupJob.perform_now([ "orphan-id" ], snapshot)

    assert_requested delete_stub
    assert_requested revoke
  end

  test "a missing id list still revokes the grant" do
    account = connect_google!(@david)
    snapshot = account.cleanup_snapshot
    revoke = stub_google_revoke
    account.destroy!

    Calendar::DisconnectCleanupJob.perform_now([], snapshot)

    assert_not_requested :delete, %r{\A#{Regexp.escape(GOOGLE_EVENTS_URL)}/}
    assert_requested revoke
  end

  test "blank credentials are a no-op" do
    Calendar::DisconnectCleanupJob.perform_now([ "orphan-id" ], nil)

    assert_not_requested :delete, %r{\A#{Regexp.escape(GOOGLE_EVENTS_URL)}/}
    assert_not_requested :post, GOOGLE_REVOKE_URL
  end
end
