# WebMock lifecycle for system tests that stub external APIs (Drive,
# Fizzy, GitHub, X, Calendar). Include it instead of hand-rolling the
# enable/reset/disable blocks: the teardown order below is load-bearing.
#
# webmock/minitest clears every stub in Minitest's #teardown phase, which
# runs before Rails' `teardown do` blocks (those run later, inside
# #after_teardown) and before Capybara drains the app server's pending
# requests (also in #after_teardown, via reset_sessions!). So a browser
# fetch that is still running on the Puma thread when the body ends would
# look its stub up after the stubs are gone and fail the test with
# WebMock::NetConnectNotAllowedError. Navigating away in a teardown block
# cannot fix that: it runs after the reset.
#
# before_teardown runs before #teardown, while the stubs are still
# registered, so this module leaves the page and waits out the pending
# server requests here. super runs first so a failed test still
# screenshots the real page, not the blank one.
module WebMockSystemTestHelper
  extend ActiveSupport::Concern

  included do
    setup :enable_webmock_for_system_test
    teardown :reset_webmock_for_system_test
  end

  def before_teardown
    super
    begin
      visit "about:blank"
    ensure
      page.server&.wait_for_pending_requests
    end
  end

  # Belt and suspenders around the teardown above: WebMock must never leak
  # out of a stubbed file, even when a test or an earlier teardown step
  # errors. The browser's HTTP client is shared across tests (see
  # ApplicationSystemTestCase), so leaving WebMock enabled here breaks every
  # later system test's chromedriver traffic.
  def after_teardown
    super
  ensure
    WebMock.reset!
    WebMock.disable!
  end

  private
    def enable_webmock_for_system_test
      WebMock.enable!
      WebMock.disable_net_connect!(allow_localhost: true)
    end

    def reset_webmock_for_system_test
      WebMock.reset!
      WebMock.disable!
    end
end
