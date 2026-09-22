require "test_helper"

class Calendar::RemoteDeleteJobTest < ActiveSupport::TestCase
  include GoogleCalendarTestHelper

  setup do
    @david = users(:david)
  end

  test "deletes the remote copy with the user's current credentials" do
    account = connect_google!(@david)
    delete_stub = stub_google_event_delete("orphan-id")

    Calendar::RemoteDeleteJob.perform_now(@david.id, "orphan-id")

    assert_requested delete_stub, headers: { "Authorization" => "Bearer #{account.access_token}" }
  end

  test "skips a missing account without a request" do
    Calendar::RemoteDeleteJob.perform_now(@david.id, "orphan-id")

    assert_not_requested :delete, "#{GOOGLE_EVENTS_URL}/orphan-id"
  end

  test "skips a disconnected account without a request" do
    connect_google!(@david, disconnected_reason: "Google rejected the connection")

    Calendar::RemoteDeleteJob.perform_now(@david.id, "orphan-id")

    assert_not_requested :delete, "#{GOOGLE_EVENTS_URL}/orphan-id"
  end

  test "skips a grant without the calendar scope" do
    connect_google!(@david, scopes: "openid email")

    Calendar::RemoteDeleteJob.perform_now(@david.id, "orphan-id")

    assert_not_requested :delete, "#{GOOGLE_EVENTS_URL}/orphan-id"
  end

  test "treats gone and revoked copies as success" do
    connect_google!(@david)
    missing_delete = stub_google_event_delete("missing-id", status: 404)
    gone_delete = stub_google_event_delete("gone-id", status: 410)

    Calendar::RemoteDeleteJob.perform_now(@david.id, "missing-id")
    Calendar::RemoteDeleteJob.perform_now(@david.id, "gone-id")

    assert_requested missing_delete
    assert_requested gone_delete
  end

  test "an invalid_grant disconnects and counts as success" do
    account = connect_google!(@david)
    account.update!(access_token_expires_at: 1.hour.ago)
    stub_google_token_invalid_grant

    Calendar::RemoteDeleteJob.perform_now(@david.id, "orphan-id")

    assert_equal "Google rejected the connection", account.reload.disconnected_reason
    assert_not_requested :delete, "#{GOOGLE_EVENTS_URL}/orphan-id"
  end

  test "a transient failure schedules a retry" do
    connect_google!(@david)
    stub_request(:delete, "#{GOOGLE_EVENTS_URL}/orphan-id").to_timeout

    assert_enqueued_with(job: Calendar::RemoteDeleteJob, args: [ @david.id, "orphan-id" ]) do
      Calendar::RemoteDeleteJob.perform_now(@david.id, "orphan-id")
    end
  end

  test "other Google failures are logged without raising or retrying" do
    connect_google!(@david)
    stub_google_event_delete("orphan-id", status: 500)

    assert_no_enqueued_jobs(only: Calendar::RemoteDeleteJob) do
      Calendar::RemoteDeleteJob.perform_now(@david.id, "orphan-id")
    end
  end
end
