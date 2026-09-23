require "application_system_test_case"

# The policy is enforced, so a violation both breaks the page feature and
# fires securitypolicyviolation; these tests make sure the main flows
# raise none. A listener installed before any page script records every
# event in sessionStorage, which survives the full page loads between
# steps.
class ContentSecurityPolicyTest < ApplicationSystemTestCase
  include GoogleCalendarTestHelper

  LIVEKIT_ENV = %w[ LIVEKIT_URL LIVEKIT_INTERNAL_URL LIVEKIT_API_KEY LIVEKIT_API_SECRET LIVEKIT_GATEWAY_SECRET ].freeze
  PICKER_ENV = %w[ GOOGLE_PICKER_API_KEY GOOGLE_CLOUD_PROJECT_NUMBER ].freeze

  RECORD_VIOLATIONS_JS = <<~JS
    document.addEventListener("securitypolicyviolation", (event) => {
      try {
        const seen = JSON.parse(sessionStorage.getItem("cspViolations") || "[]");
        seen.push({ directive: event.effectiveDirective, blocked: event.blockedURI, page: location.pathname,
                    source: event.sourceFile, line: event.lineNumber, sample: event.sample });
        sessionStorage.setItem("cspViolations", JSON.stringify(seen));
      } catch (error) {}
    });
  JS

  driven_by :selenium, using: :headless_chrome, screen_size: [ 1400, 1000 ], options: { name: :csp_chrome } do |options|
    options.add_argument "--use-fake-device-for-media-stream"
    options.add_argument "--use-fake-ui-for-media-stream"
    # Keep the fake microphone's beep (and any remote audio) off the
    # host speakers; capture and Web Audio analysis are unaffected.
    options.add_argument "--mute-audio"
  end

  setup do
    @env_before_test = (LIVEKIT_ENV + PICKER_ENV).index_with { |name| ENV[name] }

    # A LiveKit address nothing listens on: the huddle panel renders and its
    # client tries to connect, without a real server.
    ENV["LIVEKIT_URL"] = "ws://127.0.0.1:9"
    ENV["LIVEKIT_INTERNAL_URL"] = "http://127.0.0.1:10"
    ENV["LIVEKIT_API_KEY"] = "csp-test-key"
    ENV["LIVEKIT_API_SECRET"] = "csp-test-secret-csp-test-secret-0000"
    ENV["LIVEKIT_GATEWAY_SECRET"] = "csp-test-gateway-secret"
    ENV["GOOGLE_PICKER_API_KEY"] = "test-picker-key"
    ENV["GOOGLE_CLOUD_PROJECT_NUMBER"] = "123456789012"

    page.driver.browser.execute_cdp("Page.addScriptToEvaluateOnNewDocument", source: RECORD_VIOLATIONS_JS)
    visit root_url
    page.execute_script("sessionStorage.removeItem('cspViolations')")
  end

  teardown do
    @env_before_test.each { |name, value| ENV[name] = value }
  end

  test "signing in through the real login form lands on the signed-in page" do
    # Every other system test signs in through the test-only route, so this
    # is the one test that submits the real login form end to end.
    visit new_session_url
    fill_in "email_address", with: "jz@37signals.com"
    fill_in "password", with: "secret123456"
    click_on "log_in"
    # The login POST plus the first room render (with the huddle panel this
    # file enables) exceeds the default wait under parallel load, and a
    # specific room row is not the point: wait for the signed-in sidebar
    # and the workspace user instead.
    assert_selector "aside#sidebar nav[aria-label='Your workspace']", wait: 10
    assert_selector "aside#sidebar a.workspace-user", text: "JZ", wait: 10
    assert_no_violations
  end

  test "sign-in, a room, markdown, the Drive picker composer, and the huddle panel raise no violations" do
    # The fast sign-in helper skips the login form; load it explicitly so
    # the sign-in page keeps its violation coverage.
    visit new_session_url
    assert_field "email_address"
    sign_in "jz@37signals.com"
    assert Huddle.configured?
    join_room rooms(:designers)

    assert_selector "[data-controller~='drive-share']", visible: :all
    assert_selector "#channel-huddle", visible: :all

    send_message "CSP check with **bold**, `code`, and a list:\n\n- one\n- two\n\n```ruby\nputs 1\n```"
    assert_selector ".message__body strong", text: "bold"
    assert_selector ".message__body pre", text: "puts 1"

    click_button "Join huddle"
    assert_selector "#channel-huddle:not([hidden])", wait: 10
    if page.has_css?("#channel-huddle[data-state='prejoin']", wait: 5)
      assert_selector "[data-huddle-target='checkJoin']:not([disabled])", wait: 20
      find("[data-huddle-target='checkJoin']").click
    end
    # The connection to the closed LiveKit port fails; give it time to try.
    assert_selector "#channel-huddle[data-state='failed']", visible: :all, wait: 15

    visit user_profile_url
    assert_selector "h2", text: "GitHub"

    visit new_room_event_url(rooms(:designers))
    assert_selector "form"

    assert_no_violations
  end

  test "a Turbo visit to the event form fills the time zone with no violations" do
    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    page.execute_script("window.cspTurboMarker = true")
    page.execute_script("Turbo.visit(arguments[0])", new_room_event_path(rooms(:designers)))
    assert_selector "form[data-controller~='event-time-zone']"
    assert page.evaluate_script("window.cspTurboMarker === true"), "the visit stayed a Turbo visit"

    # The Stimulus controller (not an inline script, so no nonce is
    # needed) fills the hidden field and its label from the browser's
    # time zone. The label assertion retries until the lazily loaded
    # controller runs; it sets the field first, so the field is filled
    # by the time the label matches.
    browser_zone = page.evaluate_script("Intl.DateTimeFormat().resolvedOptions().timeZone")
    assert_predicate browser_zone, :present?
    assert_selector "[data-event-time-zone-target='label']", text: browser_zone
    assert_equal browser_zone, find("[data-event-time-zone-target='field']", visible: :all).value

    assert_no_violations
  end

  test "a forced inline script without the nonce is blocked and reported" do
    sign_in "jz@37signals.com"
    join_room rooms(:designers)

    page.execute_script(<<~JS)
      const script = document.createElement("script");
      script.textContent = "window.cspProbeRan = true";
      document.body.append(script);
    JS

    violations = wait_for_violations
    assert violations.any? { |violation| violation["directive"].to_s.start_with?("script-src") },
      "expected the nonce-less script to be reported, got #{violations.inspect}"
    assert_not page.evaluate_script("window.cspProbeRan === true"), "expected the nonce-less script to be blocked"
  end

  test "room, huddle, stage, board, search, settings, and the profile card raise no violations" do
    david = users(:david)
    stage = Rooms::Stage.create_for({ name: "Town Hall", creator: david }, users: [ david ])
    board = Rooms::Board.create_for({ name: "Launch", creator: david }, users: [ david ])

    sign_in "david@37signals.com"

    join_room rooms(:designers)
    click_button "Join huddle"
    assert_selector "#channel-huddle:not([hidden])", wait: 10
    if page.has_css?("#channel-huddle[data-state='prejoin']", wait: 5)
      assert_selector "[data-huddle-target='checkJoin']:not([disabled])", wait: 20
      find("[data-huddle-target='checkJoin']").click
    end
    # The connection to the closed LiveKit port fails; give it time to try.
    assert_selector "#channel-huddle[data-state='failed']", visible: :all, wait: 15

    join_room stage
    assert_selector ".room--current", text: "Town Hall"

    join_room board
    assert_selector ".board__header h1", text: "Launch"

    visit searches_path
    assert_selector "#message-area"

    visit edit_account_path
    assert_selector "#account_users"

    join_room rooms(:designers)
    within "#message_#{messages(:first).client_message_id}" do
      find(".message__avatar a").click
    end
    assert_selector "#profile-card-popover:not([hidden])", wait: 10
    within "#user_card" do
      assert_selector ".profile-card__name", text: "Jason"
    end

    assert_no_violations
  end

  private
    def recorded_violations
      JSON.parse(page.evaluate_script("sessionStorage.getItem('cspViolations') || '[]'"))
    end

    def wait_for_violations
      deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + 5
      violations = recorded_violations
      while violations.empty? && Process.clock_gettime(Process::CLOCK_MONOTONIC) < deadline
        sleep 0.1
        violations = recorded_violations
      end
      violations
    end

    def assert_no_violations
      violations = recorded_violations
      assert_empty violations, "CSP violations: #{JSON.pretty_generate(violations)}"
    end
end
