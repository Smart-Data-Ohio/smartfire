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
    settle_noise_sync

    assert_empty page.evaluate_script("window.__processorCalls")
    assert_empty page.evaluate_script("window.__restartCalls")
  end

  test "toggling noise suppression off re-acquires the microphone with browser suppression" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    install_stub_room(noise_enabled: true, processor: "campfire-rnnoise")

    find("[data-huddle-target='noise']").click

    wait_for_condition("toggling off did not stop the noise processor") do
      page.evaluate_script("window.__processorCalls.length") > 0
    end
    wait_for_condition("toggling off did not re-acquire the microphone") do
      page.evaluate_script("window.__restartCalls.length") > 0
    end

    assert_equal [ [ "stop" ] ], page.evaluate_script("window.__processorCalls")
    assert_equal(
      {
        "noiseSuppression" => true, "echoCancellation" => true, "autoGainControl" => true,
        "voiceIsolation" => true, "deviceId" => "stub-device"
      },
      page.evaluate_script("window.__restartCalls").last
    )
  end

  test "toggling noise suppression on re-acquires the microphone without browser suppression" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    install_stub_room(noise_enabled: false)

    find("[data-huddle-target='noise']").click

    wait_for_condition("toggling on did not attach the noise processor") do
      page.evaluate_script("window.__processorCalls.length") > 0
    end
    wait_for_condition("toggling on did not re-acquire the microphone") do
      page.evaluate_script("window.__restartCalls.length") > 0
    end

    assert_equal [ [ "set", "campfire-rnnoise" ] ], page.evaluate_script("window.__processorCalls")
    assert_equal(
      {
        "noiseSuppression" => false, "echoCancellation" => true, "autoGainControl" => true,
        "voiceIsolation" => true, "deviceId" => "stub-device"
      },
      page.evaluate_script("window.__restartCalls").last
    )
  end

  test "toggling noise suppression off without a stored device re-acquires on the live device" do
    visit room_path(rooms(:designers))
    wait_for_cable_connection
    install_stub_room(noise_enabled: true, processor: "campfire-rnnoise")
    page.execute_script("window.__audioTrack._constraints = { noiseSuppression: false, echoCancellation: true, autoGainControl: true }")

    find("[data-huddle-target='noise']").click

    wait_for_condition("toggling off did not re-acquire the microphone") do
      page.evaluate_script("window.__restartCalls.length") > 0
    end

    assert_equal(
      {
        "noiseSuppression" => true, "echoCancellation" => true, "autoGainControl" => true,
        "voiceIsolation" => true, "deviceId" => { "ideal" => "live-device" }
      },
      page.evaluate_script("window.__restartCalls").last
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
    settle_noise_sync
    assert_empty page.evaluate_script("window.__restartCalls")

    # The stopped track rejects the update while muted; unmuting retries
    # it against the live track.
    toggle_mute_and_await(2, "unmuting did not reach the room")
    wait_for_condition("unmuting did not re-acquire the microphone") do
      page.evaluate_script("window.__restartCalls.length") > 0
    end

    assert_equal(
      {
        "noiseSuppression" => true, "echoCancellation" => true, "autoGainControl" => true,
        "voiceIsolation" => true, "deviceId" => "stub-device"
      },
      page.evaluate_script("window.__restartCalls").last
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

    # The noise sync runs on a queue behind the toggle that triggered it, so
    # "nothing happened" assertions settle it first instead of racing it.
    def settle_noise_sync
      page.driver.browser.execute_async_script(<<~JS)
        (window.__huddleController.noiseOperation || Promise.resolve())
          .then(() => arguments[arguments.length - 1](true));
      JS
    end

    # A connected panel with a stubbed room: setMicrophoneEnabled records
    # its arguments and flips, the mic publication carries a stub audio
    # track that records processor, restart, and constraint calls. Restarts
    # and constraint updates reject while muted, like a stopped live track,
    # and the stored constraints start as the SDK's own capture would leave
    # them: the full set on the actual device.
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
        window.__restartCalls = [];
        window.__constraintCalls = [];
        window.__audioCaptureDefaults = null;
        let micEnabled = true;
        let processor = arguments[1] ? { name: arguments[1] } : null;
        const audioTrack = {
          _constraints: {
            noiseSuppression: !arguments[0],
            echoCancellation: true,
            autoGainControl: true,
            voiceIsolation: true,
            deviceId: "stub-device"
          },
          get constraints() { return this._constraints; },
          get isMuted() { return !micEnabled; },
          getSourceTrackSettings: () => ({ deviceId: "live-device", channelCount: 1 }),
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
          restartTrack: (constraints) => {
            window.__restartCalls.push({ ...constraints });
            if (!micEnabled) return Promise.reject(new DOMException("Test stopped track", "InvalidStateError"));
            audioTrack._constraints = { ...constraints };
            return Promise.resolve();
          },
          applyConstraints: (constraints) => {
            if (!micEnabled) return Promise.reject(new DOMException("Test stopped track", "InvalidStateError"));
            window.__constraintCalls.push({ ...constraints });
            audioTrack._constraints = { ...audioTrack._constraints, ...constraints };
            return Promise.resolve();
          }
        };
        window.__audioTrack = audioTrack;
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
