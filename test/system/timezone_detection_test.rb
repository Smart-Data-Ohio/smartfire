require "application_system_test_case"

class TimezoneDetectionTest < ApplicationSystemTestCase
  test "the browser does not report its zone without a CSRF token" do
    with_forgery_protection(false) do
      sign_in "jz@37signals.com"
      join_room rooms(:designers)

      # Forget any detected zone so the next page load reports again; the
      # report below must not happen without a token to verify it with.
      users(:jz).update!(time_zone: nil)
      visit room_url(rooms(:watercooler))

      assert_zone_never_reported users(:jz)
    end
  end

  test "the browser reports its detected zone once" do
    with_forgery_protection(true) do
      sign_in "jz@37signals.com"
      join_room rooms(:designers)

      wait_for_detected_zone users(:jz)
      assert_equal page.evaluate_script("Intl.DateTimeFormat().resolvedOptions().timeZone"),
        users(:jz).reload.time_zone
    end
  end

  private
    def with_forgery_protection(enabled)
      original = ActionController::Base.allow_forgery_protection
      ActionController::Base.allow_forgery_protection = enabled
      yield
    ensure
      ActionController::Base.allow_forgery_protection = original
    end

    # Waits until the timezone controller has connected (a report would
    # dispatch synchronously from connect), then asserts no report lands
    # within a settle window. The controller reports at most once per page
    # load, so a clean window proves it stayed silent.
    def assert_zone_never_reported(user, settle: 2)
      page.document.synchronize(10) do
        connected = page.evaluate_script(
          "!!window.Stimulus?.getControllerForElementAndIdentifier(document.body, 'timezone')")
        raise Capybara::ExpectationNotMet, "waiting for the timezone controller" unless connected
      end

      deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + settle
      while Process.clock_gettime(Process::CLOCK_MONOTONIC) < deadline
        flunk "expected no time zone report without a CSRF token" if user.reload.time_zone.present?
        sleep 0.1
      end
      assert_nil user.reload.time_zone
    end

    def wait_for_detected_zone(user, timeout: 10)
      deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + timeout
      until user.reload.time_zone.present?
        flunk "expected the browser to report its time zone" if Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline
        sleep 0.1
      end
    end
end
