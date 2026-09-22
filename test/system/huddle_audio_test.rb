require "application_system_test_case"

# Audio processing behavior with a stubbed room: which capture constraints
# the mute toggle requests, when the noise processor attaches, and which
# defaults unmuting re-acquires from. The LiveKit suite covers the same
# flows against a real room; these run everywhere.
class HuddleAudioTest < ApplicationSystemTestCase
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

  test "the capture asks for no browser suppression while RNNoise is on" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    install_stub_room(noise_enabled: true)

    toggle_mute_and_await(1, "muting did not reach the room")
    toggle_mute_and_await(2, "unmuting did not reach the room")

    mutes = page.evaluate_script("window.__micCalls")
    assert_equal [ false, true ], mutes.map { |call| call["enabling"] }
    assert_equal [ false, false ], mutes.map { |call| noise_option(call) }
  end

  test "the capture keeps browser suppression while RNNoise is off" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    install_stub_room(noise_enabled: false)

    toggle_mute_and_await(1, "muting did not reach the room")
    toggle_mute_and_await(2, "unmuting did not reach the room")

    mutes = page.evaluate_script("window.__micCalls")
    assert_equal [ false, true ], mutes.map { |call| call["enabling"] }
    assert_equal [ true, true ], mutes.map { |call| noise_option(call) }
  end

  test "unmuting requests the selected device without rewriting the room defaults" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    page.execute_script("window.localStorage.setItem('campfire.huddle.devices', JSON.stringify({ audioinput: 'mic-1' }))")
    install_stub_room(noise_enabled: false)

    toggle_mute_and_await(1, "muting did not reach the room")
    toggle_mute_and_await(2, "unmuting did not reach the room")

    mutes = page.evaluate_script("window.__micCalls")
    assert_equal({ "ideal" => "mic-1" }, mutes.last["deviceId"])
    # Unmuting re-acquires from the track's stored constraints, so the
    # controller leaves the room defaults alone.
    assert_nil page.evaluate_script("window.__audioCaptureDefaults")
  end

  test "muting keeps the noise processor attached across mute and unmute" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    install_stub_room(noise_enabled: true, processor: "campfire-rnnoise")

    toggle_mute_and_await(1, "muting did not reach the room")
    toggle_mute_and_await(2, "unmuting did not reach the room")
    wait_for_condition("the noise sync did not run after unmuting") do
      page.evaluate_script("window.__constraintCalls.length") > 0
    end

    assert_empty page.evaluate_script("window.__processorCalls")
  end

  test "toggling noise suppression off restores browser suppression on the live track" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    install_stub_room(noise_enabled: true, processor: "campfire-rnnoise")

    find("[data-huddle-target='noise']").click

    wait_for_condition("toggling off did not stop the noise processor") do
      page.evaluate_script("window.__processorCalls.length") > 0
    end
    wait_for_condition("toggling off did not update the capture constraints") do
      page.evaluate_script("window.__constraintCalls.length") > 0
    end

    assert_equal [ [ "stop" ] ], page.evaluate_script("window.__processorCalls")
    assert_equal(
      { "noiseSuppression" => true, "echoCancellation" => true, "autoGainControl" => true },
      page.evaluate_script("window.__constraintCalls").last
    )
  end

  test "toggling noise suppression on disables browser suppression on the live track" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    install_stub_room(noise_enabled: false)

    find("[data-huddle-target='noise']").click

    wait_for_condition("toggling on did not attach the noise processor") do
      page.evaluate_script("window.__processorCalls.length") > 0
    end
    wait_for_condition("toggling on did not update the capture constraints") do
      page.evaluate_script("window.__constraintCalls.length") > 0
    end

    assert_equal [ [ "set", "campfire-rnnoise" ] ], page.evaluate_script("window.__processorCalls")
    assert_equal(
      { "noiseSuppression" => false, "echoCancellation" => true, "autoGainControl" => true },
      page.evaluate_script("window.__constraintCalls").last
    )
  end

  test "toggling noise suppression off while muted applies on unmute" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    install_stub_room(noise_enabled: true, processor: "campfire-rnnoise")

    toggle_mute_and_await(1, "muting did not reach the room")
    find("[data-huddle-target='noise']").click
    wait_for_condition("toggling off did not stop the noise processor") do
      page.evaluate_script("window.__processorCalls.length") > 0
    end

    # The stopped track rejects the update while muted; unmuting retries
    # it against the live track.
    toggle_mute_and_await(2, "unmuting did not reach the room")
    wait_for_condition("unmuting did not update the capture constraints") do
      page.evaluate_script("window.__constraintCalls.length") > 0
    end

    assert_equal(
      { "noiseSuppression" => true, "echoCancellation" => true, "autoGainControl" => true },
      page.evaluate_script("window.__constraintCalls").last
    )
  end

  private
    def click_mute_button
      find("[data-huddle-target='mute']").click
    end

    def toggle_mute_and_await(call_count, message)
      click_mute_button
      wait_for_condition(message) { (page.evaluate_script("window.__micCalls.length") || 0) >= call_count }
    end

    def wait_for_condition(message)
      deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + 10
      until yield
        flunk message if Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline
        sleep 0.05
      end
      assert true
    end

    # A connected panel with a stubbed room: setMicrophoneEnabled records
    # its arguments and flips, the mic publication carries a stub audio
    # track that records processor and constraint calls. Constraint updates
    # reject while muted, like a stopped live track.
    def install_stub_room(noise_enabled:, processor: nil)
      page.execute_script(<<~JS, noise_enabled, processor)
        const controller = window.Stimulus
          .getControllerForElementAndIdentifier(document.getElementById("channel-huddle"), "huddle");
        window.__huddleController = controller;
        controller.liveKit = { Track: { Source: { Microphone: "microphone" } } };
        controller.noiseSuppressionAvailable = true;
        controller.noiseSuppressionEnabled = arguments[0];
        controller.roomId = 1;
        controller.state = "connected";
        controller.canPublish = true;
        window.__micCalls = [];
        window.__processorCalls = [];
        window.__constraintCalls = [];
        window.__audioCaptureDefaults = null;
        let micEnabled = true;
        let processor = arguments[1] ? { name: arguments[1] } : null;
        const audioTrack = {
          setProcessor: (next) => {
            window.__processorCalls.push([ "set", next?.name || "unidentified" ]);
            processor = next;
            return Promise.resolve();
          },
          stopProcessor: () => {
            window.__processorCalls.push([ "stop" ]);
            processor = null;
            return Promise.resolve();
          },
          getProcessor: () => processor,
          applyConstraints: (constraints) => {
            if (!micEnabled) return Promise.reject(new DOMException("Test stopped track", "InvalidStateError"));
            window.__constraintCalls.push({ ...constraints });
            return Promise.resolve();
          }
        };
        const participant = {
          identity: "fake",
          name: "Jason",
          isSpeaking: false,
          get isMicrophoneEnabled() { return micEnabled; },
          setMicrophoneEnabled: (enabling, options) => {
            window.__micCalls.push({
              enabling: enabling,
              noiseSuppression: options?.noiseSuppression,
              deviceId: options?.deviceId
            });
            micEnabled = enabling;
            return Promise.resolve();
          },
          getTrackPublication: (source) => source === "microphone" ? { audioTrack } : null
        };
        controller.room = {
          options: {},
          localParticipant: participant,
          remoteParticipants: new Map()
        };
        const originalSetMicrophoneEnabled = participant.setMicrophoneEnabled;
        participant.setMicrophoneEnabled = (enabling, options) => {
          const result = originalSetMicrophoneEnabled(enabling, options);
          if (enabling) window.__audioCaptureDefaults = controller.room.options.audioCaptureDefaults || null;
          return result;
        };
        controller.element.hidden = false;
        controller.activeControlsTarget.hidden = false;
        controller.settingsTarget.hidden = false;
        controller.settingsRowTarget.hidden = false;
        controller.muteTarget.disabled = false;
        controller.noiseTarget.disabled = false;
      JS
    end

    # The capture constraints the toggle requested, defaulting to the
    # browser's own suppression when it passed nothing explicit.
    def noise_option(call)
      call["noiseSuppression"].nil? ? true : call["noiseSuppression"]
    end
end
