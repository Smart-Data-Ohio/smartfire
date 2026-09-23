require "test_helper"

class Calendar::MeetingRefreshTest < ActiveSupport::TestCase
  include GoogleCalendarTestHelper

  setup do
    @user = users(:david)
    @user.update!(meeting_status_enabled: true)
    @account = connect_google!(@user)
  end

  test "a successful refresh stores busy intervals and clears the error" do
    Calendar::MeetingCache.create!(user: @user, fetch_error: "stale notice")
    stub_list_events(items: [
      timed_item("2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z"),
      timed_item("2026-09-23T13:00:00Z", "2026-09-23T13:30:00Z", "transparency" => "transparent")
    ])

    assert_equal :ok, Calendar::MeetingRefresh.refresh(@user.id, now: Time.zone.parse("2026-09-23T10:30:00Z"))

    cache = @user.reload.meeting_cache
    assert_equal [ [ "2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z" ] ], cache.busy_intervals
    assert_equal Time.zone.parse("2026-09-23T10:30:00Z"), cache.fetched_at
    assert_nil cache.fetch_error
  end

  test "only free/busy fields are requested: no titles or attendee identities" do
    stub = stub_list_events(items: [])

    Calendar::MeetingRefresh.refresh(@user.id)

    assert_requested stub
    requested_fields = Google::Client::MEETING_STATUS_FIELDS
    assert_includes requested_fields, "eventType"
    assert_not_includes requested_fields, "summary"
    assert_not_includes requested_fields, "description"
    assert_not_includes requested_fields, "email"
    assert_not_includes requested_fields, "displayName"
  end

  test "a member who never opted in is skipped without a request" do
    @user.update!(meeting_status_enabled: false)
    stub = stub_list_events(items: [])

    assert_equal :skipped, Calendar::MeetingRefresh.refresh(@user.id)

    assert_not_requested stub
    assert_nil @user.reload.meeting_cache
  end

  test "a deactivated member is skipped without a request" do
    stub = stub_list_events(items: [])
    @user.update!(status: :deactivated)

    assert_equal :skipped, Calendar::MeetingRefresh.refresh(@user.id)

    assert_not_requested stub
  end

  test "a member without a usable account records a connect notice and clears intervals" do
    @account.update!(disconnected_reason: "Google rejected the connection")
    Calendar::MeetingCache.create!(user: @user,
      busy_intervals: [ [ "2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z" ] ])
    stub = stub_list_events(items: [])

    assert_equal :error, Calendar::MeetingRefresh.refresh(@user.id, now: Time.zone.parse("2026-09-23T10:30:00Z"))

    assert_not_requested stub
    cache = @user.reload.meeting_cache
    assert_empty cache.busy_intervals
    assert_equal Calendar::MeetingRefresh::NOT_CONNECTED_MESSAGE, cache.fetch_error
    assert_equal Time.zone.parse("2026-09-23T10:30:00Z"), cache.fetched_at
  end

  test "a revoked grant records a reconnect notice and turns the status off" do
    @account.update!(access_token_expires_at: 1.hour.ago)
    Calendar::MeetingCache.create!(user: @user,
      busy_intervals: [ [ "2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z" ] ])
    stub_google_token_invalid_grant

    assert_equal :error, Calendar::MeetingRefresh.refresh(@user.id)

    cache = @user.reload.meeting_cache
    assert_empty cache.busy_intervals
    assert_equal Calendar::MeetingRefresh::RECONNECT_MESSAGE, cache.fetch_error
    assert_equal "Google rejected the connection", @account.reload.disconnected_reason
  end

  test "rate limits keep the last good intervals and record a retry notice" do
    busy = [ [ "2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z" ] ]
    Calendar::MeetingCache.create!(user: @user, busy_intervals: busy)
    stub_request(:get, %r{\A#{GOOGLE_EVENTS_URL}})
      .to_return(status: 429, body: {}.to_json)

    assert_equal :error, Calendar::MeetingRefresh.refresh(@user.id)

    cache = @user.reload.meeting_cache
    assert_equal busy, cache.busy_intervals
    assert_equal Calendar::MeetingRefresh::UNREACHABLE_MESSAGE, cache.fetch_error
    assert_not_nil cache.fetched_at
  end

  test "a quota 403 keeps the last good intervals and records a retry notice" do
    busy = [ [ "2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z" ] ]
    Calendar::MeetingCache.create!(user: @user, busy_intervals: busy)
    stub_request(:get, %r{\A#{GOOGLE_EVENTS_URL}})
      .to_return(status: 403, body: google_forbidden_body("userRateLimitExceeded").to_json)

    assert_equal :error, Calendar::MeetingRefresh.refresh(@user.id)

    cache = @user.reload.meeting_cache
    assert_equal busy, cache.busy_intervals
    assert_equal Calendar::MeetingRefresh::UNREACHABLE_MESSAGE, cache.fetch_error
  end

  test "a server error keeps the last good intervals and records a retry notice" do
    busy = [ [ "2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z" ] ]
    Calendar::MeetingCache.create!(user: @user, busy_intervals: busy)
    stub_request(:get, %r{\A#{GOOGLE_EVENTS_URL}})
      .to_return(status: 500, body: "boom")

    assert_equal :error, Calendar::MeetingRefresh.refresh(@user.id)

    cache = @user.reload.meeting_cache
    assert_equal busy, cache.busy_intervals
    assert_equal Calendar::MeetingRefresh::UNREACHABLE_MESSAGE, cache.fetch_error
  end

  test "a malformed response body keeps the last good intervals and records a retry notice" do
    busy = [ [ "2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z" ] ]
    Calendar::MeetingCache.create!(user: @user, busy_intervals: busy)
    stub_request(:get, %r{\A#{GOOGLE_EVENTS_URL}})
      .to_return(status: 200, body: "{oops")

    assert_equal :error, Calendar::MeetingRefresh.refresh(@user.id)

    cache = @user.reload.meeting_cache
    assert_equal busy, cache.busy_intervals
    assert_equal Calendar::MeetingRefresh::UNREACHABLE_MESSAGE, cache.fetch_error
  end

  test "a JSON parse failure escaping the client is recorded instead of raising" do
    busy = [ [ "2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z" ] ]
    Calendar::MeetingCache.create!(user: @user, busy_intervals: busy)
    Google::Client.any_instance.stubs(:list_events).raises(JSON::ParserError.new("unexpected token"))

    assert_equal :error, Calendar::MeetingRefresh.refresh(@user.id)

    cache = @user.reload.meeting_cache
    assert_equal busy, cache.busy_intervals
    assert_equal Calendar::MeetingRefresh::UNREACHABLE_MESSAGE, cache.fetch_error
  end

  test "a fresh cache is not refetched" do
    Calendar::MeetingCache.create!(user: @user, fetched_at: 30.seconds.ago)
    stub = stub_list_events(items: [])

    assert_equal :fresh, Calendar::MeetingRefresh.refresh(@user.id)

    assert_not_requested stub
  end

  test "a throttled refresh enqueues one delayed follow-up" do
    Calendar::MeetingCache.create!(user: @user, fetched_at: 30.seconds.ago)
    stub = stub_list_events(items: [])

    assert_equal :fresh, Calendar::MeetingRefresh.refresh(@user.id)

    assert_not_requested stub
    followups = ActiveJob::Base.queue_adapter.enqueued_jobs
      .select { |job| job[:job] == Calendar::MeetingRefreshJob }
    assert_equal 1, followups.size
    assert_equal [ @user.id ], followups.first[:args]
    assert_in_delta Calendar::MeetingRefresh::PUSH_THROTTLE.from_now.to_f, followups.first[:at], 5
  end

  test "a second throttled refresh inside the window enqueues no further follow-up" do
    Calendar::MeetingCache.create!(user: @user, fetched_at: 30.seconds.ago)

    assert_equal :fresh, Calendar::MeetingRefresh.refresh(@user.id)

    assert_no_enqueued_jobs only: Calendar::MeetingRefreshJob do
      assert_equal :fresh, Calendar::MeetingRefresh.refresh(@user.id)
    end
  end

  test "a completed fetch clears the follow-up claim" do
    Calendar::MeetingCache.create!(user: @user, fetched_at: 30.seconds.ago)
    stub_list_events(items: [])

    assert_equal :fresh, Calendar::MeetingRefresh.refresh(@user.id)
    travel 61.seconds do
      assert_equal :ok, Calendar::MeetingRefresh.refresh(@user.id)
    end

    assert_enqueued_with(job: Calendar::MeetingRefreshJob, args: [ @user.id ]) do
      assert_equal :fresh, Calendar::MeetingRefresh.refresh(@user.id)
    end
  end

  test "an OOO-only member's refresh stores OOO intervals with the wider lookahead" do
    @user.update!(meeting_status_enabled: false, ooo_calendar_enabled: true)
    stub_list_events(items: [
      timed_item("2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z", "eventType" => "outOfOffice"),
      timed_item("2026-09-23T13:00:00Z", "2026-09-23T13:30:00Z")
    ])
    now = Time.zone.parse("2026-09-23T10:30:00Z")

    assert_equal :ok, Calendar::MeetingRefresh.refresh(@user.id, now:)

    cache = @user.reload.meeting_cache
    assert_equal [ [ "2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z" ] ], cache.ooo_intervals
    assert_empty cache.busy_intervals
    assert_requested :get, GOOGLE_EVENTS_URL,
      query: hash_including("timeMax" => (now + 30.days).iso8601)
  end

  test "a member with both opt-ins stores both interval sets" do
    @user.update!(ooo_calendar_enabled: true)
    stub_list_events(items: [
      timed_item("2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z", "eventType" => "outOfOffice"),
      timed_item("2026-09-23T13:00:00Z", "2026-09-23T13:30:00Z")
    ])

    assert_equal :ok, Calendar::MeetingRefresh.refresh(@user.id, now: Time.zone.parse("2026-09-23T10:30:00Z"))

    cache = @user.reload.meeting_cache
    assert_equal [ [ "2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z" ] ], cache.ooo_intervals
    assert_equal [ [ "2026-09-23T13:00:00Z", "2026-09-23T13:30:00Z" ] ], cache.busy_intervals
  end

  test "a meeting-only member stores no OOO intervals" do
    stub_list_events(items: [
      timed_item("2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z", "eventType" => "outOfOffice")
    ])

    assert_equal :ok, Calendar::MeetingRefresh.refresh(@user.id)

    cache = @user.reload.meeting_cache
    assert_empty cache.ooo_intervals
    assert_empty cache.busy_intervals
  end

  test "a meeting-only member fetches the meeting lookahead" do
    stub_list_events(items: [])
    now = Time.zone.parse("2026-09-23T10:30:00Z")

    Calendar::MeetingRefresh.refresh(@user.id, now:)

    assert_requested :get, GOOGLE_EVENTS_URL,
      query: hash_including("timeMax" => (now + 24.hours).iso8601)
  end

  test "a server error keeps OOO intervals too" do
    @user.update!(ooo_calendar_enabled: true)
    ooo = [ [ "2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z" ] ]
    Calendar::MeetingCache.create!(user: @user, ooo_intervals: ooo)
    stub_request(:get, %r{\A#{GOOGLE_EVENTS_URL}})
      .to_return(status: 500, body: "boom")

    assert_equal :error, Calendar::MeetingRefresh.refresh(@user.id)

    cache = @user.reload.meeting_cache
    assert_equal ooo, cache.ooo_intervals
    assert_equal Calendar::MeetingRefresh::UNREACHABLE_MESSAGE, cache.fetch_error
  end

  test "a revoked grant clears both meeting and OOO intervals" do
    @user.update!(ooo_calendar_enabled: true, meeting_status_enabled: true)
    @account.update!(access_token_expires_at: 1.hour.ago)
    Calendar::MeetingCache.create!(user: @user,
      busy_intervals: [ [ "2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z" ] ],
      ooo_intervals: [ [ "2026-09-23T10:00:00Z", "2026-09-23T11:00:00Z" ] ])
    stub_google_token_invalid_grant

    assert_equal :error, Calendar::MeetingRefresh.refresh(@user.id)

    cache = @user.reload.meeting_cache
    assert_empty cache.busy_intervals
    assert_empty cache.ooo_intervals
    assert_equal Calendar::MeetingRefresh::RECONNECT_MESSAGE, cache.fetch_error
  end

  private
    def stub_list_events(items:)
      stub_google_events_list(items:)
    end

    def timed_item(start_at, end_at, **attrs)
      timed_calendar_item(Time.zone.parse(start_at), Time.zone.parse(end_at), **attrs)
    end
end
