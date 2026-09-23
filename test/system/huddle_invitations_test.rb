require "application_system_test_case"
require "timeout"

class HuddleInvitationsTest < ApplicationSystemTestCase
  setup do
    @environment_names = Huddle::REQUIRED_ENVIRONMENT
    @original_livekit_environment = ENV.values_at(*@environment_names)
    ENV["LIVEKIT_URL"] = "wss://huddle.example.test"
    ENV["LIVEKIT_INTERNAL_URL"] = "ws://livekit.example.test:7880"
    ENV["LIVEKIT_API_KEY"] = "test-api-key"
    ENV["LIVEKIT_API_SECRET"] = "test-api-secret"
    ENV["LIVEKIT_GATEWAY_SECRET"] = "test-gateway-secret"
    sign_in "jason@37signals.com"
  end

  teardown do
    @environment_names.zip(@original_livekit_environment).each { |name, value| ENV[name] = value }
  end

  test "the recipient sees an incoming huddle banner and dismissing it marks the item read" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection

    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))
    item = ActivityItem.find_by!(user: users(:jason), source: grant)

    assert_selector "#huddle-invitation:not([hidden])", text: "David started a huddle", wait: 10
    assert_selector "#huddle-invitation:not([hidden])", text: "Join the huddle in David"
    assert_selector ".workspace-activity-count", text: "1", wait: 10

    within "#huddle-invitation" do
      click_button "Dismiss"
    end

    assert_selector "#huddle-invitation[hidden]", visible: :all, wait: 10
    assert_selector ".workspace-activity-count[hidden]", visible: :all, wait: 10
    assert_predicate item.reload, :read?
  end

  test "joining from the banner marks the item handled, navigates to the DM room, and rings the huddle panel" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    page.execute_script("window.huddleJoinEvents = []; window.addEventListener('huddle:join', event => window.huddleJoinEvents.push(event.detail))")

    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))
    room = grant.room
    item = ActivityItem.find_by!(user: users(:jason), source: grant)

    assert_selector "#huddle-invitation:not([hidden])", wait: 10

    within "#huddle-invitation" do
      click_button "Join"
    end

    assert_current_path room_path(room), wait: 10
    assert_selector ".room--current", text: "David"

    Timeout.timeout(Capybara.default_max_wait_time) do
      sleep 0.05 until page.evaluate_script("window.huddleJoinEvents.length") > 0
    end
    assert_equal [ { "roomId" => room.id, "roomName" => "David" } ], page.evaluate_script("window.huddleJoinEvents")
    assert_predicate item.reload, :handled?
  end

  test "the banner flips to caller-left when the starter hangs up, then dismisses" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection

    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))
    item = ActivityItem.find_by!(user: users(:jason), source: grant)
    assert_selector "#huddle-invitation:not([hidden])", text: "David started a huddle", wait: 10

    grant.update_columns(last_seen_at: Time.current)
    grant.revoke!

    assert_selector "#huddle-invitation:not([hidden])", text: "David left the huddle", wait: 10
    assert_selector "#huddle-invitation:not([hidden])", text: "Missed call in David"
    assert_selector "#huddle-invitation[hidden]", visible: :all, wait: 10
    assert_equal "huddle_started", item.reload.event_type
  end

  test "a ring stops itself after the ring timeout" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    page.execute_script(<<~JS)
      document.getElementById("huddle-invitation")
        .setAttribute("data-huddle-invitation-ring-timeout-value", "300")
    JS

    HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))

    assert_selector "#huddle-invitation:not([hidden])", text: "David started a huddle", wait: 10
    assert_selector "#huddle-invitation[hidden]", visible: :all, wait: 10
  end

  test "a banner-only ring stops itself after the ring timeout" do
    users(:jason).update!(inbox_preferences: { "huddle_invitations" => false })
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    page.execute_script(<<~JS)
      document.getElementById("huddle-invitation")
        .setAttribute("data-huddle-invitation-ring-timeout-value", "300")
    JS

    HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))

    assert_selector "#huddle-invitation:not([hidden])", text: "David started a huddle", wait: 10
    assert_selector "#huddle-invitation[hidden]", visible: :all, wait: 10
    assert_not ActivityItem.exists?(user: users(:jason))
  end

  test "the banner stays hidden while already in the room's huddle" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection

    room = rooms(:david_and_jason)
    page.execute_script(<<~JS, room.id)
      window.dispatchEvent(new CustomEvent("huddle:changed", {
        detail: { roomId: arguments[0], state: "connected" }
      }))
    JS

    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))
    item = ActivityItem.find_by!(user: users(:jason), source: grant)

    assert_no_selector "#huddle-invitation:not([hidden])", wait: 5
    assert_selector "#huddle-invitation[hidden]", visible: :all
    assert_predicate item.reload, :unread?
  end

  test "an incoming call rings audibly until it is answered" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection

    HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))

    assert_selector "#huddle-invitation:not([hidden])", text: "David started a huddle", wait: 10
    assert invitation_ringing_wanted?, "expected the banner to want its ringtone"

    # The first real click unlocks audio; the ring starts on it. The title
    # carries no action, so the click only counts as a gesture.
    find("#huddle-invitation [data-huddle-invitation-target='title']").click
    wait_for_condition("the ringtone never started") { invitation_ringing? }

    within "#huddle-invitation" do
      click_button "Dismiss"
    end

    assert_selector "#huddle-invitation[hidden]", visible: :all, wait: 10
    assert_not invitation_ringing_wanted?
    assert_not invitation_ringing?
  end

  test "a silent invitation shows the banner without any sound" do
    Huddle::RingPolicy.quiet_check = ->(_user) { true }
    begin
      visit room_path(rooms(:designers))
      wait_for_cable_connection

      HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))

      assert_selector "#huddle-invitation:not([hidden])", text: "David started a huddle", wait: 10
      assert_not invitation_ringing_wanted?
      assert_not invitation_ringing?
    ensure
      Huddle::RingPolicy.quiet_check = nil
    end
  end

  test "a hidden tab raises a system notification for the call" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    page.execute_script(<<~JS)
      window.__notifications = [];
      window.__nativeNotification = window.Notification;
      window.__NotificationStub = class {
        static permission = "granted";
        constructor(title, options) {
          window.__notifications.push({ title, options });
          this.closed = false;
        }
        close() { this.closed = true; }
      };
      window.Notification = window.__NotificationStub;
      Object.defineProperty(document, "visibilityState", { configurable: true, get: () => "hidden" });
    JS

    begin
      HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))

      assert_selector "#huddle-invitation:not([hidden])", text: "David started a huddle", wait: 10
      wait_for_condition("no system notification was raised") do
        page.evaluate_script("window.__notifications.length") >= 1
      end

      notification = page.evaluate_script("window.__notifications[0]")
      assert_equal "David started a huddle", notification["title"]
      assert_match "Join the huddle in David", notification["options"]["body"]
    ensure
      page.execute_script(<<~JS)
        window.Notification = window.__nativeNotification;
        delete document.visibilityState;
      JS
    end
  end

  test "the ring stops when the caller leaves" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection

    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))
    assert_selector "#huddle-invitation:not([hidden])", text: "David started a huddle", wait: 10
    assert invitation_ringing_wanted?

    grant.update_columns(last_seen_at: Time.current)
    grant.revoke!

    assert_selector "#huddle-invitation:not([hidden])", text: "David left the huddle", wait: 10
    assert_not invitation_ringing_wanted?
    assert_not invitation_ringing?
  end

  private
    def invitation_controller_script(expression)
      page.evaluate_script(<<~JS, expression)
        window.Stimulus.getControllerForElementAndIdentifier(
          document.getElementById("huddle-invitation"), "huddle-invitation")[arguments[0]]
      JS
    end

    def invitation_ringing_wanted?
      invitation_controller_script("shouldRing") == true
    end

    def invitation_ringing?
      invitation_controller_script("ringing") == true
    end

    def wait_for_condition(message, timeout: 10)
      deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + timeout
      until yield
        flunk message if Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline
        sleep 0.05
      end
      assert true
    end
end
