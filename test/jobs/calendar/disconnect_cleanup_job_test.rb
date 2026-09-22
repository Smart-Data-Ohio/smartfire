require "test_helper"

class Calendar::DisconnectCleanupJobTest < ActiveSupport::TestCase
  include GoogleCalendarTestHelper

  setup do
    @david = users(:david)
  end

  test "deletes every remote copy then revokes the grant" do
    account = connect_google!(@david)
    access_token = account.access_token
    refresh_token = account.refresh_token
    snapshot = account.cleanup_snapshot
    first_delete = stub_google_event_delete("first-id")
    second_delete = stub_google_event_delete("second-id")
    revoke = stub_google_revoke
    account.destroy!

    Calendar::DisconnectCleanupJob.perform_now([ "first-id", "second-id" ], snapshot)

    assert_requested first_delete, headers: { "Authorization" => "Bearer #{access_token}" }
    assert_requested second_delete
    assert_requested revoke, body: hash_including({ "token" => refresh_token })
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

  test "permanent delete failures are best effort and still revoke" do
    account = connect_google!(@david)
    snapshot = account.cleanup_snapshot
    delete_stub = stub_google_event_delete("orphan-id", status: 500)
    revoke = stub_google_revoke
    account.destroy!

    assert_no_enqueued_jobs(only: Calendar::DisconnectCleanupJob) do
      Calendar::DisconnectCleanupJob.perform_now([ "orphan-id" ], snapshot)
    end

    assert_requested delete_stub
    assert_requested revoke
  end

  test "a refresh 503 retries without revoking until the deletes succeed" do
    account = connect_google!(@david)
    account.update!(access_token_expires_at: 1.hour.ago)
    snapshot = account.cleanup_snapshot
    recovered_token = "recovered-access-token"
    stub_request(:post, GOOGLE_TOKEN_URL).to_return(status: 503)
    delete_stub = stub_google_event_delete("orphan-id")
    revoke = stub_google_revoke
    account.destroy!

    assert_enqueued_with(job: Calendar::DisconnectCleanupJob, args: [ [ "orphan-id" ], snapshot ]) do
      Calendar::DisconnectCleanupJob.perform_now([ "orphan-id" ], snapshot)
    end

    assert_not_requested delete_stub
    assert_not_requested revoke

    stub_google_token_refresh(access_token: recovered_token)

    Calendar::DisconnectCleanupJob.perform_now([ "orphan-id" ], snapshot)

    assert_requested delete_stub, headers: { "Authorization" => "Bearer #{recovered_token}" }
    assert_requested revoke
  end

  test "a transient delete failure schedules a retry without revoking" do
    account = connect_google!(@david)
    snapshot = account.cleanup_snapshot
    delete_stub = stub_google_event_delete("orphan-id", status: 429)
    revoke = stub_google_revoke
    account.destroy!

    assert_enqueued_with(job: Calendar::DisconnectCleanupJob, args: [ [ "orphan-id" ], snapshot ]) do
      Calendar::DisconnectCleanupJob.perform_now([ "orphan-id" ], snapshot)
    end

    assert_requested delete_stub
    assert_not_requested revoke
  end

  test "a revoke 5xx schedules a retry" do
    account = connect_google!(@david)
    snapshot = account.cleanup_snapshot
    delete_stub = stub_google_event_delete("orphan-id")
    revoke = stub_google_revoke(status: 500)
    account.destroy!

    assert_enqueued_with(job: Calendar::DisconnectCleanupJob, args: [ [ "orphan-id" ], snapshot ]) do
      Calendar::DisconnectCleanupJob.perform_now([ "orphan-id" ], snapshot)
    end

    assert_requested delete_stub
    assert_requested revoke
  end

  test "an exhausted retry logs the failure at error level" do
    account = connect_google!(@david)
    snapshot = account.cleanup_snapshot
    stub_google_event_delete("orphan-id", status: 429)
    revoke = stub_google_revoke
    account.destroy!

    job = Calendar::DisconnectCleanupJob.new([ "orphan-id" ], snapshot)
    job.exception_executions = { [ Google::Client::Unavailable ].to_s => 8 }

    log = capture_job_logs do
      assert_no_enqueued_jobs(only: Calendar::DisconnectCleanupJob) do
        job.perform_now
      end
    end

    assert_includes log, "Calendar::DisconnectCleanupJob failed after retries"
    assert_not_requested revoke
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

  test "an unreadable credentials blob logs a warning with the account id" do
    account = connect_google!(@david)
    access_token = account.access_token
    refresh_token = account.refresh_token
    blob = account.cleanup_snapshot
    account_id = account.id
    tampered = blob.dup
    tampered[10] = (tampered[10] == "A" ? "B" : "A")
    account.destroy!

    log = capture_job_logs do
      Calendar::DisconnectCleanupJob.perform_now([ "orphan-id" ], tampered, account_id)
    end

    assert_includes log, "account #{account_id}"
    assert_not_includes log, access_token
    assert_not_includes log, refresh_token
    assert_not_requested :delete, %r{\A#{Regexp.escape(GOOGLE_EVENTS_URL)}/}
    assert_not_requested :post, GOOGLE_REVOKE_URL
  end

  test "tokens never reach the job logs" do
    account = connect_google!(@david)
    access_token = account.access_token
    refresh_token = account.refresh_token
    blob = account.cleanup_snapshot
    account.destroy!

    assert_not_includes blob.inspect, access_token
    assert_not_includes blob.inspect, refresh_token
    assert_not_predicate Calendar::DisconnectCleanupJob, :log_arguments?

    stub_google_event_delete("orphan-id")
    stub_google_revoke

    log = capture_job_logs do
      Calendar::DisconnectCleanupJob.perform_later([ "orphan-id" ], blob)
      perform_enqueued_jobs only: Calendar::DisconnectCleanupJob
    end

    assert_not_includes log, access_token
    assert_not_includes log, refresh_token
  end

  private
    def capture_job_logs
      io = StringIO.new
      logger = ActiveSupport::TaggedLogging.new(ActiveSupport::Logger.new(io))
      old_rails_logger = Rails.logger
      old_job_logger = ActiveJob::Base.logger
      old_subscriber_logger = ActiveJob::LogSubscriber.logger
      Rails.logger = logger
      ActiveJob::Base.logger = logger
      ActiveJob::LogSubscriber.logger = logger
      yield
      io.string
    ensure
      Rails.logger = old_rails_logger
      ActiveJob::Base.logger = old_job_logger
      ActiveJob::LogSubscriber.logger = old_subscriber_logger
    end
end
