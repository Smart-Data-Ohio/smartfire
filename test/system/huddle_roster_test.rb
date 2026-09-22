require "application_system_test_case"

# In-call rendering and polling with a stubbed room: roster rows patch in
# place across speaking, mute, and membership changes, and the microphone
# meter stops at the end of its track and skips ticks while hidden. The
# LiveKit suite covers the same flows against a real room; these run
# everywhere.
class HuddleRosterTest < ApplicationSystemTestCase
  driven_by :selenium, using: :headless_chrome, screen_size: [ 1400, 1400 ] do |options|
    options.add_argument("--use-fake-device-for-media-stream")
    options.add_argument("--use-fake-ui-for-media-stream")
  end

  setup do
    @original_livekit_environment = ENV.values_at(*Huddle::REQUIRED_ENVIRONMENT)
    ENV["LIVEKIT_URL"] = "wss://huddle.example.test"
    ENV["LIVEKIT_INTERNAL_URL"] = "ws://livekit.example.test:7880"
    ENV["LIVEKIT_API_KEY"] = "test-api-key"
    ENV["LIVEKIT_API_SECRET"] = "test-api-secret"
    ENV["LIVEKIT_GATEWAY_SECRET"] = "test-gateway-secret"

    sign_in "jason@37signals.com"
  end

  teardown do
    Huddle::REQUIRED_ENVIRONMENT.zip(@original_livekit_environment).each do |name, value|
      ENV[name] = value
    end
  end

  test "muting patches the local roster row instead of rebuilding it" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    install_stub_room
    toggle_mute_and_await(1, "muting did not reach the room")
    assert_selector "li[data-participant-identity='local-1']", text: "Muted"

    mark_roster_row("local-1")
    toggle_mute_and_await(2, "unmuting did not reach the room")
    assert_selector "li[data-participant-identity='local-1']", text: "Listening"

    assert_roster_row_kept("local-1")
    assert_equal "Listening", roster_activity("local-1")
    assert roster_label("local-1").end_with?(", Listening")
    assert_selector "li[data-participant-identity='local-1'] .huddle__participant-name", text: "(you)"
  end

  test "speaking and mute changes patch the remote row in place" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    install_stub_room
    toggle_mute_and_await(1, "muting did not reach the room")
    assert_selector "li[data-participant-identity='remote-1']", text: "Listening"

    mark_roster_row("remote-1")
    page.execute_script(<<~JS)
      window.__remote.isSpeaking = true;
      window.__remoteMic.isMuted = true;
    JS
    toggle_mute_and_await(2, "unmuting did not reach the room")
    assert_selector "li[data-participant-identity='remote-1']", text: "Speaking"

    assert_roster_row_kept("remote-1")
    assert_equal "Speaking", roster_activity("remote-1")
    assert roster_row_class("remote-1").include?("huddle__participant--speaking")

    page.execute_script(<<~JS)
      window.__remote.isSpeaking = false;
    JS
    toggle_mute_and_await(3, "muting did not reach the room")
    assert_selector "li[data-participant-identity='remote-1']", text: "Muted"

    assert_roster_row_kept("remote-1")
    assert_equal "Muted", roster_activity("remote-1")
    assert_not_includes roster_row_class("remote-1"), "huddle__participant--speaking"
  end

  test "joining and leaving adds and removes roster rows only" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    install_stub_room
    toggle_mute_and_await(1, "muting did not reach the room")
    assert_selector "li[data-participant-identity='remote-1']", text: "Listening"

    mark_roster_row("local-1")
    page.execute_script(<<~JS)
      const remoteMic = { isMuted: false };
      window.__remoteParticipants.set("remote-2", {
        identity: "remote-2",
        name: "Alice",
        isSpeaking: false,
        trackPublications: new Map(),
        getTrackPublication: (source) => source === "microphone" ? remoteMic : null
      });
    JS
    toggle_mute_and_await(2, "unmuting did not reach the room")

    assert_roster_row_kept("local-1")
    assert_selector "li[data-participant-identity='remote-2']", text: "Alice"
    assert_equal "3 participants", page.evaluate_script(<<~JS)
      document.querySelector("[data-huddle-target='participantCount']").textContent
    JS

    page.execute_script(<<~JS)
      window.__remoteParticipants.delete("remote-1");
    JS
    toggle_mute_and_await(3, "muting did not reach the room")

    assert_roster_row_kept("local-1")
    assert_no_selector "li[data-participant-identity='remote-1']"
    assert_selector "li[data-participant-identity='remote-2']", text: "Alice"
  end

  test "the mute toggle keeps a stable label with pressed state and tooltip" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    install_stub_room

    assert_toggle_state "mute", label: "Mute microphone", pressed: false, tooltip: "Microphone live"

    toggle_mute_and_await(1, "muting did not reach the room")
    assert_toggle_state "mute", label: "Mute microphone", pressed: true, tooltip: "Microphone muted"

    toggle_mute_and_await(2, "unmuting did not reach the room")
    assert_toggle_state "mute", label: "Mute microphone", pressed: false, tooltip: "Microphone live"
  end

  test "the camera toggle keeps a stable label with pressed state and tooltip" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    install_stub_room

    assert_toggle_state "camera", label: "Camera", pressed: false, tooltip: "Camera off"

    toggle_camera_and_await(1, "enabling the camera did not reach the room")
    assert_toggle_state "camera", label: "Camera", pressed: true, tooltip: "Camera on"

    toggle_camera_and_await(2, "disabling the camera did not reach the room")
    assert_toggle_state "camera", label: "Camera", pressed: false, tooltip: "Camera off"
  end

  test "the meter stops once its track ends" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    install_stub_room
    toggle_mute_and_await(1, "muting did not reach the room")
    toggle_mute_and_await(2, "unmuting did not reach the room")

    wait_for_condition("the meter never started") { meter_running? }
    assert_operator meter_samples, :>, 0

    page.execute_script("window.__mediaStreamTrack.readyState = 'ended'")
    wait_for_condition("the meter kept polling past the end of the track") { !meter_running? }
    assert page.evaluate_script("window.__meterCleanedUp")

    samples_at_stop = meter_samples
    sleep 0.3
    assert_equal samples_at_stop, meter_samples
  end

  test "the meter skips ticks while the tab is hidden" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    install_stub_room
    toggle_mute_and_await(1, "muting did not reach the room")
    toggle_mute_and_await(2, "unmuting did not reach the room")

    wait_for_condition("the meter never started") { meter_running? }

    begin
      page.execute_script(<<~JS)
        Object.defineProperty(document, "visibilityState", { configurable: true, get: () => "hidden" });
      JS

      samples_while_hidden = meter_samples
      sleep 0.35
      assert_equal samples_while_hidden, meter_samples
      assert meter_running?, "expected the meter to keep its interval while hidden"
    ensure
      page.execute_script("delete document.visibilityState")
    end

    wait_for_condition("the meter did not resume when visible") do
      meter_samples > samples_while_hidden
    end
  end

  test "leaving reports after the disconnect completes" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    install_stub_room
    page.execute_script(<<~JS)
      window.__leaveOrder = []
      // Forgery protection is off in tests, so the layout renders no CSRF
      // meta tags; the leave report needs one to attempt its POST.
      const csrfMeta = document.createElement("meta")
      csrfMeta.name = "csrf-token"
      csrfMeta.content = "test-csrf-token"
      document.head.appendChild(csrfMeta)
      const nativeLeaveFetch = window.fetch.bind(window)
      window.fetch = (...args) => {
        const input = args[0]
        const url = typeof input === "string" ? input : input.url
        if (new URL(url, window.location.origin).pathname.endsWith("/huddle/leave")) {
          window.__leaveOrder.push("reported")
        }
        return nativeLeaveFetch(...args)
      }
      window.__huddleController.room.disconnect = () => {
        window.__leaveOrder.push("disconnected")
        return Promise.resolve()
      }
    JS

    find("[data-action='huddle#leave']").click

    wait_for_condition("leaving did not disconnect and report") do
      page.evaluate_script("window.__leaveOrder.length") >= 2
    end
    assert_equal [ "disconnected", "reported" ], page.evaluate_script("window.__leaveOrder")
  end

  private
    def click_mute_button
      find("[data-huddle-target='mute']").click
    end

    def toggle_mute_and_await(call_count, message)
      click_mute_button
      wait_for_condition(message) { (page.evaluate_script("window.__micCalls.length") || 0) >= call_count }
    end

    def toggle_camera_and_await(call_count, message)
      find("[data-huddle-target='camera']").click
      wait_for_condition(message) { (page.evaluate_script("window.__cameraCalls.length") || 0) >= call_count }
    end

    def assert_toggle_state(target, label:, pressed:, tooltip:)
      assert_selector "[data-huddle-target='#{target}'][aria-pressed='#{pressed}'][title='#{tooltip}']", text: label
    end

    def wait_for_condition(message)
      deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + 10
      until yield
        flunk message if Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline
        sleep 0.05
      end
      assert true
    end

    # A connected panel with a stubbed room: a local and a remote
    # participant, a mic publication carrying a stub audio track, and a
    # recording audio analyser behind the microphone meter.
    def install_stub_room
      page.execute_script(<<~JS)
        const controller = window.Stimulus
          .getControllerForElementAndIdentifier(document.getElementById("channel-huddle"), "huddle");
        window.__huddleController = controller;
        controller.liveKit = {
          Track: { Source: { Microphone: "microphone" } },
          createAudioAnalyser: () => ({
            calculateVolume: () => {
              window.__meterSamples.push(Date.now());
              return window.__meterVolume;
            },
            cleanup: () => { window.__meterCleanedUp = true; }
          })
        };
        controller.roomId = 1;
        controller.state = "connected";
        controller.canPublish = true;
        controller.noiseSuppressionAvailable = false;
        controller.noiseSuppressionEnabled = false;
        window.__micCalls = [];
        window.__cameraCalls = [];
        window.__meterSamples = [];
        window.__meterVolume = 0.4;
        window.__meterCleanedUp = false;
        let micEnabled = true;
        let cameraEnabled = false;
        const mediaStreamTrack = { readyState: "live" };
        window.__mediaStreamTrack = mediaStreamTrack;
        const audioTrack = { mediaStreamTrack };
        const participant = {
          identity: "local-1",
          name: "Jason",
          isSpeaking: false,
          get isMicrophoneEnabled() { return micEnabled; },
          setMicrophoneEnabled: (enabling) => {
            window.__micCalls.push(enabling);
            micEnabled = enabling;
            return Promise.resolve();
          },
          get isCameraEnabled() { return cameraEnabled; },
          setCameraEnabled: (enabling) => {
            window.__cameraCalls.push(enabling);
            cameraEnabled = enabling;
            return Promise.resolve();
          },
          trackPublications: new Map(),
          getTrackPublication: (source) => source === "microphone" ? { audioTrack } : null
        };
        const remoteMic = { isMuted: false };
        const remote = {
          identity: "remote-1",
          name: "David",
          isSpeaking: false,
          trackPublications: new Map(),
          getTrackPublication: (source) => source === "microphone" ? remoteMic : null
        };
        window.__remote = remote;
        window.__remoteMic = remoteMic;
        window.__remoteParticipants = new Map([[ "remote-1", remote ]]);
        controller.room = {
          options: {},
          localParticipant: participant,
          remoteParticipants: window.__remoteParticipants
        };
        controller.element.hidden = false;
        controller.activeControlsTarget.hidden = false;
        controller.peopleTarget.hidden = false;
        controller.muteTarget.disabled = false;
        controller.cameraTarget.disabled = false;
      JS
    end

    # Marks a roster row so the test can tell a patched row from a rebuilt
    # one: a rebuild drops the mark and the tooltip with the node.
    def mark_roster_row(identity)
      page.execute_script(<<~JS, identity)
        const row = document.querySelector(`li[data-participant-identity='${arguments[0]}']`);
        row.dataset.probe = "kept";
        row.title = "hover card";
      JS
    end

    def assert_roster_row_kept(identity)
      mark = page.evaluate_script(<<~JS, identity)
        (() => {
          const row = document.querySelector(`li[data-participant-identity='${arguments[0]}']`);
          return row ? [ row.dataset.probe, row.title ] : null;
        })()
      JS
      assert_equal [ "kept", "hover card" ], mark, "expected the #{identity} row to survive the re-render"
    end

    def roster_activity(identity)
      page.evaluate_script(<<~JS, identity)
        document.querySelector(`li[data-participant-identity='${arguments[0]}'] .huddle__participant-activity`).textContent
      JS
    end

    def roster_label(identity)
      page.evaluate_script(<<~JS, identity)
        document.querySelector(`li[data-participant-identity='${arguments[0]}']`).getAttribute("aria-label")
      JS
    end

    def roster_row_class(identity)
      page.evaluate_script(<<~JS, identity)
        document.querySelector(`li[data-participant-identity='${arguments[0]}']`).className
      JS
    end

    def meter_running?
      page.evaluate_script("window.__huddleController.microphoneMeter?.running === true")
    end

    def meter_samples
      page.evaluate_script("window.__meterSamples.length")
    end
end
