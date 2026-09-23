require "test_helper"

class Calendar::InboundSyncJobTest < ActiveSupport::TestCase
  include GoogleCalendarTestHelper

  setup do
    @room = rooms(:designers)
    @event = events(:launch_party)
    @david = users(:david)
    connect_google!(@david)
    @google_id = Calendar::EntrySync.google_event_id_for(@event.id, @david.id)
    EventCalendarEntry.create!(event: @event, user: @david, google_event_id: @google_id, synced_at: Time.current)
  end

  test "a copy cancelled in Google declines the event locally" do
    @event.respond!(@david, "going")
    stub_request(:get, "#{GOOGLE_EVENTS_URL}/#{@google_id}")
      .to_return(status: 200, body: { id: @google_id, status: "cancelled" }.to_json,
        headers: { "Content-Type" => "application/json" })

    Calendar::InboundSyncJob.perform_now(@david.id)

    assert_equal "declined", @event.reload.response_for(@david)
  end

  test "a copy deleted in Google declines the event locally" do
    @event.respond!(@david, "maybe")
    stub_request(:get, "#{GOOGLE_EVENTS_URL}/#{@google_id}").to_return(status: 404)

    Calendar::InboundSyncJob.perform_now(@david.id)

    assert_equal "declined", @event.reload.response_for(@david)
  end

  test "a confirmed copy never flips a local decline back to going" do
    @event.respond!(@david, "declined")
    stub_request(:get, "#{GOOGLE_EVENTS_URL}/#{@google_id}")
      .to_return(status: 200, body: { id: @google_id, status: "confirmed" }.to_json,
        headers: { "Content-Type" => "application/json" })

    Calendar::InboundSyncJob.perform_now(@david.id)

    assert_equal "declined", @event.reload.response_for(@david)
  end

  test "a confirmed copy leaves a going response alone" do
    @event.respond!(@david, "going")
    stub_request(:get, "#{GOOGLE_EVENTS_URL}/#{@google_id}")
      .to_return(status: 200, body: { id: @google_id, status: "confirmed" }.to_json,
        headers: { "Content-Type" => "application/json" })

    assert_no_difference -> { @event.attendances.find_by(user: @david).updated_at } do
      Calendar::InboundSyncJob.perform_now(@david.id)
    end
  end

  test "cancelled events are never touched" do
    @event.respond!(@david, "going")
    @event.update!(cancelled_at: Time.current)
    get = stub_request(:get, %r{www\.googleapis\.com/calendar})

    Calendar::InboundSyncJob.perform_now(@david.id)

    assert_not_requested get
  end

  test "nothing happens without a usable account" do
    @david.google_account.mark_disconnected!("revoked")
    get = stub_request(:get, %r{www\.googleapis\.com/calendar})

    Calendar::InboundSyncJob.perform_now(@david.id)

    assert_not_requested get
  end

  test "a revoked grant aborts the sweep instead of failing every entry" do
    other = events(:watercooler_sync)
    other_id = Calendar::EntrySync.google_event_id_for(other.id, @david.id)
    EventCalendarEntry.create!(event: other, user: @david, google_event_id: other_id, synced_at: Time.current)
    gets = stub_request(:get, %r{#{GOOGLE_EVENTS_URL}/}).to_return(status: 401)
    stub_request(:post, GOOGLE_TOKEN_URL)
      .to_return(status: 400, body: { error: "invalid_grant" }.to_json)

    Calendar::InboundSyncJob.perform_now(@david.id)

    assert_requested gets, times: 1
    assert_not_predicate @david.google_account.reload, :connected?
  end

  test "the sweep preloads events instead of querying per entry" do
    watercooler_id = Calendar::EntrySync.google_event_id_for(events(:watercooler_sync).id, @david.id)
    EventCalendarEntry.create!(event: events(:watercooler_sync), user: @david,
      google_event_id: watercooler_id, synced_at: Time.current)
    third = @room.events.create!(organizer: @david, title: "Third", starts_at: 4.days.from_now, time_zone: "UTC")
    third_id = Calendar::EntrySync.google_event_id_for(third.id, @david.id)
    EventCalendarEntry.create!(event: third, user: @david, google_event_id: third_id, synced_at: Time.current)
    stub_request(:get, %r{#{GOOGLE_EVENTS_URL}/})
      .to_return(status: 200, body: { status: "confirmed" }.to_json,
        headers: { "Content-Type" => "application/json" })

    event_reads = count_sql_queries('FROM "events"') do
      Calendar::InboundSyncJob.perform_now(@david.id)
    end

    assert_equal 1, event_reads
  end

  private
    def count_sql_queries(fragment)
      queries = []
      callback = ->(*, payload) { queries << payload[:sql] if payload[:sql].include?(fragment) }
      ActiveSupport::Notifications.subscribed(callback, "sql.active_record") { yield }
      queries.size
    end
end
