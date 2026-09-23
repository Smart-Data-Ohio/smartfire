require "test_helper"

class Calendar::MeetLinkJobTest < ActiveSupport::TestCase
  include GoogleCalendarTestHelper

  setup do
    @event = events(:launch_party)
    @organizer = @event.organizer
    @google_id = Calendar::EntrySync.google_event_id_for(@event.id, @organizer.id)

    @url_host_before_test = Rails.application.routes.default_url_options[:host]
    Rails.application.routes.default_url_options[:host] = "www.example.com"
  end

  teardown do
    if @url_host_before_test.nil?
      Rails.application.routes.default_url_options.delete(:host)
    else
      Rails.application.routes.default_url_options[:host] = @url_host_before_test
    end
  end

  test "provisions a Meet link through the organizer's calendar copy" do
    connect_google!(@organizer)
    @event.update!(meet_link_requested: true)
    stub_google_event_insert
    conference = stub_request(:put, "#{GOOGLE_EVENTS_URL}/#{@google_id}?conferenceDataVersion=1")
      .with { |request| JSON.parse(request.body).dig("conferenceData", "createRequest", "requestId").present? }
      .to_return(status: 200, body: { hangoutLink: "https://meet.google.com/abc-defg-hij" }.to_json,
        headers: { "Content-Type" => "application/json" })

    Calendar::MeetLinkJob.perform_now(@event.id)

    assert_requested conference, times: 1
    assert_equal "https://meet.google.com/abc-defg-hij", @event.reload.meet_link
  end

  test "nothing happens without a request" do
    connect_google!(@organizer)
    conference = stub_request(:put, %r{www\.googleapis\.com/calendar})

    Calendar::MeetLinkJob.perform_now(@event.id)

    assert_not_requested conference
    assert_nil @event.reload.meet_link
  end

  test "no Meet link unless the organizer has connected Google" do
    @event.update!(meet_link_requested: true)
    conference = stub_request(:put, %r{www\.googleapis\.com/calendar})

    Calendar::MeetLinkJob.perform_now(@event.id)

    assert_not_requested conference
    assert_nil @event.reload.meet_link
  end

  test "a disconnected organizer account provisions nothing" do
    connect_google!(@organizer, disconnected_reason: "revoked")
    @event.update!(meet_link_requested: true)
    conference = stub_request(:put, %r{www\.googleapis\.com/calendar})

    Calendar::MeetLinkJob.perform_now(@event.id)

    assert_not_requested conference
    assert_nil @event.reload.meet_link
  end

  test "a second run is a no-op once the link exists" do
    connect_google!(@organizer)
    @event.update!(meet_link_requested: true, meet_link: "https://meet.google.com/abc-defg-hij")
    conference = stub_request(:put, %r{www\.googleapis\.com/calendar})

    Calendar::MeetLinkJob.perform_now(@event.id)

    assert_not_requested conference
  end

  test "a permanent Google refusal is logged, not raised" do
    connect_google!(@organizer)
    @event.update!(meet_link_requested: true)
    stub_google_event_insert
    stub_request(:put, "#{GOOGLE_EVENTS_URL}/#{@google_id}?conferenceDataVersion=1")
      .to_return(status: 403, body: google_forbidden_body("forbidden").to_json,
        headers: { "Content-Type" => "application/json" })

    Calendar::MeetLinkJob.perform_now(@event.id)

    assert_nil @event.reload.meet_link
    assert @event.meet_link_requested?
  end

  test "creating an event with a request enqueues provisioning" do
    assert_enqueued_with(job: Calendar::MeetLinkJob) do
      @event.room.events.create!(
        organizer: @organizer, title: "Standup", starts_at: 1.day.from_now,
        time_zone: "UTC", meet_link_requested: true
      )
    end
  end

  test "a series copies the request to every occurrence" do
    event = @event.room.events.create!(
      organizer: @organizer, title: "Standup", starts_at: Time.current.change(hour: 9) + 1.day,
      time_zone: "UTC", recurrence_rule: "weekly",
      recurrence_until: 3.weeks.from_now.to_date, meet_link_requested: true
    )

    assert event.series_events.where(meet_link_requested: true).count >= 2
  end
end
