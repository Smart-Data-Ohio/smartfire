require "application_system_test_case"

class MeetingStatusTest < ApplicationSystemTestCase
  include GoogleCalendarTestHelper

  setup do
    WebMock.enable!
    WebMock.disable_net_connect!(allow_localhost: true)
  end

  teardown do
    # Leave the page first: a rendered Drive chip or link preview can still
    # be fetching metadata through the app, and that request must not land
    # after the stubs below are reset.
    visit "about:blank"
    WebMock.reset!
    WebMock.disable!
  end

  # Belt and suspenders around the teardown above: WebMock must never leak
  # out of this file (see DriveLinkPreviewsTest).
  def after_teardown
    super
  ensure
    WebMock.reset!
    WebMock.disable!
  end

  test "opting in shows In a meeting for a stubbed busy interval, then clears after it ends" do
    connect_google!(users(:david))
    stub_google_events_list(items: [
      timed_calendar_item(5.minutes.ago, 5.minutes.from_now),
      timed_calendar_item(6.minutes.ago, 4.minutes.ago, "transparency" => "transparent")
    ])
    sign_in "david@37signals.com"
    visit user_profile_url

    find("#user_meeting_status_enabled", visible: :all).ancestor("label").click
    within("form[action='#{user_status_path}']") { find("button[type='submit']", match: :first).click }
    wait_for_condition("meeting status was not enabled") { users(:david).reload.meeting_status_enabled? }

    perform_enqueued_jobs only: Calendar::MeetingRefreshJob
    Calendar::MeetingDispatcher.dispatch_due!

    visit user_url(users(:david))
    assert_selector ".user-status-badge", text: "In a meeting"

    travel_to 6.minutes.from_now do
      Calendar::MeetingDispatcher.dispatch_due!

      visit user_url(users(:david))
      assert_no_selector ".user-status-badge__custom", text: "In a meeting"
    end
  end

  test "the profile links to connect without a Google account" do
    sign_in "david@37signals.com"
    visit user_profile_url

    assert_selector "a[href='#google-calendar-title']", text: "Connect Google Calendar"
    assert_no_selector "#user_meeting_status_enabled", visible: :all
  end

  private
    def wait_for_condition(message, timeout: 10)
      deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + timeout
      until yield
        flunk message if Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline
        sleep 0.05
      end
      assert true
    end
end
