require "application_system_test_case"

# Call controls with a stubbed room: push-to-talk, the mute shortcut,
# per-participant volumes and mutes, the reconnection UI, connection-quality
# badges, speaking rings, and microphone recovery. The LiveKit suite covers
# the same flows against a real room; these run everywhere.
class CallControlsTest < ApplicationSystemTestCase
  self.use_transactional_tests = false

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
    @created_rooms = []
    @sessions = [ "default" ]

    sign_in "jason@37signals.com"
  end

  teardown do
    begin
      @created_rooms.each(&:destroy!)
      HuddleCleanup.delete_all
      HuddleGrant.delete_all
    ensure
      Huddle::REQUIRED_ENVIRONMENT.zip(@original_livekit_environment).each do |name, value|
        ENV[name] = value
      end
    end
  end

  test "push-to-talk opens the microphone while held and never while typing" do
    room = rooms(:designers)
    visit room_path(room)
    wait_for_cable_connection
    install_stub_room(room.id)
    page.execute_script(<<~JS)
      window.__huddleController.voiceModeValue = "push_to_talk";
      window.__huddleController.pushToTalkKeyValue = "`";
    JS

    # Mute first: the hold opens a closed microphone.
    find("[data-huddle-target='mute']").click
    wait_for_condition("muting did not reach the room") { mic_calls.length >= 1 }

    hold_push_to_talk("`")
    wait_for_condition("the hold did not open the microphone") { mic_calls.length >= 2 }
    assert_equal [ false, true ], mic_calls

    release_push_to_talk("`")
    wait_for_condition("releasing did not close the microphone") { mic_calls.length >= 3 }
    assert_equal [ false, true, false ], mic_calls

    # Typing the key changes nothing and the keypress is not swallowed.
    prevented = page.evaluate_script(<<~JS)
      (() => {
        const probe = document.createElement("input");
        probe.id = "ptt-typing-probe";
        document.body.appendChild(probe);
        probe.focus();
        const defaulted = probe.dispatchEvent(new KeyboardEvent("keydown", { key: "`", bubbles: true, cancelable: true }));
        probe.remove();
        return !defaulted;
      })()
    JS
    assert_not prevented, "expected typing the push-to-talk key to reach the field"
    sleep 0.3
    assert_equal [ false, true, false ], mic_calls
    assert_selector "[data-huddle-target='mute'][title='Hold ` to talk']"

    # A configured key replaces the backtick, in the hold and the tooltip.
    page.execute_script("window.__huddleController.pushToTalkKeyValue = 'v'")
    hold_push_to_talk("v")
    wait_for_condition("the custom key did not open the microphone") { mic_calls.length >= 4 }
    release_push_to_talk("v")
    wait_for_condition("releasing the custom key did not close the microphone") { mic_calls.length >= 5 }

    hold_push_to_talk("`")
    sleep 0.3
    assert_equal 5, mic_calls.length
    assert_selector "[data-huddle-target='mute'][title='Hold v to talk']"
  end

  test "push-to-talk ignores composing and modified keys, releases when hidden, and matches the backtick by position" do
    room = rooms(:designers)
    visit room_path(room)
    wait_for_cable_connection
    install_stub_room(room.id)
    page.execute_script(<<~JS)
      window.__huddleController.voiceModeValue = "push_to_talk";
      window.__huddleController.pushToTalkKeyValue = "`";
    JS

    find("[data-huddle-target='mute']").click
    wait_for_condition("muting did not reach the room") { mic_calls.length >= 1 }

    # A dead key on an international layout still talks: the default matches
    # the physical key, not the typed character.
    page.execute_script(<<~JS)
      document.body.dispatchEvent(new KeyboardEvent("keydown", { key: "Dead", code: "Backquote", bubbles: true }));
    JS
    wait_for_condition("the dead key did not open the microphone") { mic_calls.length >= 2 }
    page.execute_script(<<~JS)
      document.body.dispatchEvent(new KeyboardEvent("keyup", { key: "Dead", code: "Backquote", bubbles: true }));
    JS
    wait_for_condition("releasing the dead key did not close the microphone") { mic_calls.length >= 3 }
    assert_equal [ false, true, false ], mic_calls

    # Composing text swallows the key without touching the microphone.
    page.execute_script(<<~JS)
      document.body.dispatchEvent(new KeyboardEvent("keydown", { key: "`", isComposing: true, bubbles: true }));
    JS
    sleep 0.3
    assert_equal [ false, true, false ], mic_calls

    # Modified presses are shortcuts, not talk requests.
    page.execute_script(<<~JS)
      for (const modifier of [ "ctrlKey", "metaKey", "altKey" ]) {
        document.body.dispatchEvent(new KeyboardEvent("keydown", { key: "`", [modifier]: true, bubbles: true }));
      }
    JS
    sleep 0.3
    assert_equal [ false, true, false ], mic_calls

    # Hiding the page releases a held key.
    hold_push_to_talk("`")
    wait_for_condition("the hold did not open the microphone") { mic_calls.length >= 4 }
    page.execute_script(<<~JS)
      Object.defineProperty(document, "visibilityState", { value: "hidden", configurable: true });
      document.dispatchEvent(new Event("visibilitychange"));
    JS
    wait_for_condition("hiding the page did not release the key") { mic_calls.length >= 5 }
    assert_equal [ false, true, false, true, false ], mic_calls
    page.execute_script("delete document.visibilityState")
  end

  test "ctrl-shift-M toggles the microphone from anywhere, even while typing" do
    room = rooms(:designers)
    visit room_path(room)
    wait_for_cable_connection
    install_stub_room(room.id)

    prevented = page.evaluate_script(<<~JS)
      document.body.dispatchEvent(new KeyboardEvent("keydown",
        { key: "M", ctrlKey: true, shiftKey: true, bubbles: true, cancelable: true }))
    JS
    assert_not prevented, "expected the mute chord to be handled, not typed"
    wait_for_condition("the chord did not mute") { mic_calls == [ false ] }

    page.evaluate_script(<<~JS)
      (() => {
        const probe = document.createElement("input");
        probe.id = "chord-typing-probe";
        document.body.appendChild(probe);
        probe.focus();
        probe.dispatchEvent(new KeyboardEvent("keydown",
          { key: "m", ctrlKey: true, shiftKey: true, bubbles: true, cancelable: true }));
        probe.remove();
      })()
    JS
    wait_for_condition("the chord did not unmute from a field") { mic_calls == [ false, true ] }
  end

  test "a per-participant volume applies immediately and is remembered" do
    room = rooms(:designers)
    david = users(:david)
    mapping = [ { id: david.id, name: "David", avatar_url: "", identities: [ "remote-1" ] } ]

    visit room_path(room)
    wait_for_cable_connection
    pin_participant_mapping(room.id, mapping)
    install_stub_room(room.id)
    dispatch_participant_mapping(room.id, mapping)

    slider = find("li[data-participant-identity='remote-1'] .huddle__participant-volume")
    assert_equal "100", slider.value

    set_slider(slider, 150)
    wait_for_condition("the volume never applied") { volume_calls.length >= 1 }
    assert_equal [ 1.5 ], volume_calls
    assert_equal [ "set" ], audio_context_calls
    assert_equal "150", participant_volume(david.id)

    set_slider(slider, 100)
    wait_for_condition("returning to 100 never detached the gain") { audio_context_calls.length >= 2 }
    assert_equal [ "set", "unset" ], audio_context_calls
    assert_equal [ 1.5, 1 ], volume_calls
  end

  test "boosted audio follows the speaker picker" do
    room = rooms(:designers)
    david = users(:david)
    mapping = [ { id: david.id, name: "David", avatar_url: "", identities: [ "remote-1" ] } ]

    visit room_path(room)
    wait_for_cable_connection
    pin_participant_mapping(room.id, mapping)
    install_stub_room(room.id)
    dispatch_participant_mapping(room.id, mapping)
    page.execute_script(<<~JS)
      window.__sinkCalls = [];
      const proto = window.AudioContext.prototype;
      window.__sinkHad = "setSinkId" in proto;
      window.__sinkOrig = proto.setSinkId;
      if (!proto.setSinkId) proto.setSinkId = function() { return Promise.resolve(); };
      proto.setSinkId = function(deviceId) {
        window.__sinkCalls.push(deviceId);
        return Promise.resolve();
      };
      window.localStorage.setItem("campfire.huddle.devices", JSON.stringify({ audiooutput: "speaker-2" }));
    JS

    slider = find("li[data-participant-identity='remote-1'] .huddle__participant-volume")
    set_slider(slider, 150)
    wait_for_condition("the volume never applied") { volume_calls.length >= 1 }
    assert_equal [ 1.5 ], volume_calls
    assert_equal [ "speaker-2" ], page.evaluate_script("window.__sinkCalls")

    # Switching speakers retargets the boost gain too.
    page.execute_script(<<~JS)
      const controller = window.__huddleController;
      const select = document.querySelector("[data-huddle-target='speakerSelect']");
      const option = document.createElement("option");
      option.value = "speaker-3";
      option.textContent = "Speaker 3";
      select.appendChild(option);
      select.value = "speaker-3";
      controller.room.switchActiveDevice = () => Promise.resolve();
      controller.speakerChanged();
    JS
    wait_for_condition("the switch never retargeted the boost") do
      page.evaluate_script("window.__sinkCalls") == [ "speaker-2", "speaker-3" ]
    end

    page.execute_script(<<~JS)
      const proto = window.AudioContext.prototype;
      if (window.__sinkHad) {
        proto.setSinkId = window.__sinkOrig;
      } else {
        delete proto.setSinkId;
      }
    JS
  end

  test "boost caps at 100% on a non-default speaker without context routing" do
    room = rooms(:designers)
    david = users(:david)
    mapping = [ { id: david.id, name: "David", avatar_url: "", identities: [ "remote-1" ] } ]

    visit room_path(room)
    wait_for_cable_connection
    pin_participant_mapping(room.id, mapping)
    install_stub_room(room.id)
    page.execute_script(<<~JS)
      const proto = window.AudioContext.prototype;
      window.__sinkHad = "setSinkId" in proto;
      window.__sinkOrig = proto.setSinkId;
      proto.setSinkId = undefined;
      window.localStorage.setItem("campfire.huddle.devices", JSON.stringify({ audiooutput: "speaker-2" }));
    JS
    dispatch_participant_mapping(room.id, mapping)

    slider = find("li[data-participant-identity='remote-1'] .huddle__participant-volume")
    set_slider(slider, 150)
    wait_for_condition("the capped volume never applied") { volume_calls.length >= 1 }
    assert_equal [ 1 ], volume_calls
    assert_empty audio_context_calls
    assert_match(/default speaker/, slider[:title])

    # Back on the default speaker the stored boost applies again.
    page.execute_script(<<~JS)
      const proto = window.AudioContext.prototype;
      if (window.__sinkHad) {
        proto.setSinkId = window.__sinkOrig;
      } else {
        delete proto.setSinkId;
      }
      window.localStorage.setItem("campfire.huddle.devices", JSON.stringify({ audiooutput: "" }));
    JS
    set_slider(slider, 150)
    wait_for_condition("the restored boost never engaged") { audio_context_calls == [ "set" ] }
    assert_equal [ 1, 1.5 ], volume_calls
  end

  test "a stored volume renders on the slider before it is touched" do
    room = rooms(:designers)
    david = users(:david)
    mapping = [ { id: david.id, name: "David", avatar_url: "", identities: [ "remote-1" ] } ]

    visit room_path(room)
    wait_for_cable_connection
    page.execute_script("window.localStorage.setItem('campfire.huddle.volume.#{david.id}', '50')")
    pin_participant_mapping(room.id, mapping)
    install_stub_room(room.id)
    dispatch_participant_mapping(room.id, mapping)

    slider = find("li[data-participant-identity='remote-1'] .huddle__participant-volume")
    assert_equal "50", slider.value
  end

  test "a later presence update replaces the participant mapping" do
    room = rooms(:designers)
    david = users(:david)
    mapping = [ { id: david.id, name: "David", avatar_url: "", identities: [ "remote-1" ] } ]

    visit room_path(room)
    wait_for_cable_connection
    pin_participant_mapping(room.id, mapping)
    install_stub_room(room.id)
    dispatch_participant_mapping(room.id, mapping)

    find("li[data-participant-identity='remote-1'] .huddle__participant-volume")

    # The roster follows the latest report: an update with nobody in the
    # call hides the per-person controls until a mapping names them again.
    # The pin above waits out the page-load poll so only these deliberate
    # updates can reorder the mapping mid-test.
    dispatch_participant_mapping(room.id, [])
    assert_no_selector "li[data-participant-identity='remote-1'] .huddle__participant-volume"

    dispatch_participant_mapping(room.id, mapping)
    assert_selector "li[data-participant-identity='remote-1'] .huddle__participant-volume"
  end

  test "a local mute stops one participant for this browser only" do
    room = rooms(:designers)
    david = users(:david)
    mapping = [ { id: david.id, name: "David", avatar_url: "", identities: [ "remote-1" ] } ]

    visit room_path(room)
    wait_for_cable_connection
    pin_participant_mapping(room.id, mapping)
    install_stub_room(room.id)
    dispatch_participant_mapping(room.id, mapping)

    within "li[data-participant-identity='remote-1']" do
      click_button "Mute for me"
    end
    wait_for_condition("the local mute never applied") { subscribe_calls == [ false ] }
    assert_selector "li[data-participant-identity='remote-1'] .huddle__participant-mute[aria-pressed='true']"
    assert_selector "li[data-participant-identity='remote-1']", text: "Muted"
    assert_empty mic_calls, "muting one participant must not touch the local microphone"
    assert_equal "1", page.evaluate_script("window.localStorage.getItem('campfire.huddle.localMute.#{david.id}')")

    within "li[data-participant-identity='remote-1']" do
      click_button "Mute for me"
    end
    wait_for_condition("the local unmute never applied") { subscribe_calls == [ false, true ] }
    assert_selector "li[data-participant-identity='remote-1']", text: "Listening"
    assert_nil page.evaluate_script("window.localStorage.getItem('campfire.huddle.localMute.#{david.id}')")
  end

  test "reconnecting shows a countdown with a manual reconnect" do
    room = rooms(:designers)
    visit room_path(room)
    wait_for_cable_connection
    join_with_hanging_connect(room)

    page.execute_script(<<~JS)
      window.__huddleController.room.emit(window.__livekit.RoomEvent.Reconnecting);
    JS

    assert_selector "[data-huddle-target='reconnect']:not([hidden])", text: "Connection lost"
    assert_equal "30", reconnect_countdown
    sleep 1.5
    assert_equal "29", reconnect_countdown

    page.execute_script(<<~JS, room.id)
      window.__huddleFetches = 0;
      const nativeFetch = window.fetch.bind(window);
      window.fetch = (...args) => {
        const request = args[1] || {};
        const url = new URL(typeof args[0] === "string" ? args[0] : args[0].url, window.location.origin);
        if (request.method === "POST" && url.pathname === `/rooms/${arguments[0]}/huddle`) window.__huddleFetches += 1;
        return nativeFetch(...args);
      };
    JS

    click_button "Reconnect now"

    wait_for_condition("reconnect now did not rejoin") do
      page.evaluate_script("window.__huddleFetches") >= 1
    end
    assert_selector "#channel-huddle[data-state='connecting']"
    assert_selector "[data-huddle-target='reconnect'][hidden]", visible: :all
  end

  test "a struggling participant gets a poor-connection badge" do
    room = rooms(:designers)
    visit room_path(room)
    wait_for_cable_connection
    join_with_hanging_connect(room)

    page.execute_script(<<~JS)
      const controller = window.__huddleController;
      const remote = {
        identity: "remote-1", name: "David", isSpeaking: false,
        trackPublications: new Map(), getTrackPublication: () => null
      };
      controller.room.remoteParticipants.set("remote-1", remote);
      controller.room.emit(window.__livekit.RoomEvent.ConnectionQualityChanged, "poor", remote);
    JS

    assert_selector "li[data-participant-identity='remote-1'].huddle__participant--poor-connection",
      text: "Poor connection"

    page.execute_script(<<~JS)
      const controller = window.__huddleController;
      controller.room.emit(window.__livekit.RoomEvent.ConnectionQualityChanged,
        "excellent", controller.room.remoteParticipants.get("remote-1"));
    JS

    assert_no_selector "li[data-participant-identity='remote-1'].huddle__participant--poor-connection"
  end

  test "speakers get a ring on the sidebar and header stacks" do
    room = rooms(:designers)
    david = users(:david)
    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: room.memberships.find_by!(user: david))
    grant.update_columns(last_seen_at: Time.current)
    mapping = [ { id: david.id, name: "David", avatar_url: "", identities: [ "remote-1" ] } ]

    visit room_path(room)
    wait_for_cable_connection
    assert_selector "img.voice-stack__avatar[data-user-id='#{david.id}']"
    pin_participant_mapping(room.id, mapping)
    install_stub_room(room.id)
    dispatch_participant_mapping(room.id, mapping)

    page.execute_script("window.__remote.isSpeaking = true")
    find("[data-huddle-target='mute']").click
    wait_for_condition("the speaking ring never appeared") do
      page.has_css?("img.voice-stack__avatar--speaking[data-user-id='#{david.id}']", wait: 0)
    end

    page.execute_script("window.__remote.isSpeaking = false")
    find("[data-huddle-target='mute']").click
    assert_no_selector "img.voice-stack__avatar--speaking[data-user-id='#{david.id}']"
  end

  test "a newly subscribed track picks up the remembered volume" do
    room = rooms(:designers)
    david = users(:david)
    mapping = [ { id: david.id, name: "David", avatar_url: "", identities: [ "remote-1" ] } ]

    visit room_path(room)
    wait_for_cable_connection
    pin_participant_mapping(room.id, mapping)
    join_with_hanging_connect(room)
    page.execute_script("window.localStorage.setItem('campfire.huddle.volume.#{david.id}', '150')")
    dispatch_participant_mapping(room.id, mapping)

    page.execute_script(<<~JS)
      const controller = window.__huddleController;
      window.__volumeCalls = [];
      window.__audioContextCalls = [];
      const publication = {
        isSubscribed: true,
        track: { setAudioContext: (context) => window.__audioContextCalls.push(context ? "set" : "unset") }
      };
      const remote = {
        identity: "remote-1", name: "David", isSpeaking: false,
        trackPublications: new Map(), setVolume: (volume) => window.__volumeCalls.push(volume),
        getTrackPublication: () => publication
      };
      controller.room.remoteParticipants.set("remote-1", remote);
      controller.room.emit(window.__livekit.RoomEvent.TrackSubscribed,
        { kind: "audio", attach: () => document.createElement("audio") }, publication, remote);
    JS

    wait_for_condition("the subscribed track never took the remembered volume") do
      volume_calls == [ 1.5 ]
    end
    assert_equal [ "set" ], audio_context_calls
  end

  test "volumes at or below 100 stay on the audio element" do
    room = rooms(:designers)
    david = users(:david)
    mapping = [ { id: david.id, name: "David", avatar_url: "", identities: [ "remote-1" ] } ]

    visit room_path(room)
    wait_for_cable_connection
    pin_participant_mapping(room.id, mapping)
    install_stub_room(room.id)
    dispatch_participant_mapping(room.id, mapping)

    slider = find("li[data-participant-identity='remote-1'] .huddle__participant-volume")

    set_slider(slider, 0)
    wait_for_condition("muting the slider never applied") { volume_calls == [ 0 ] }
    assert_empty audio_context_calls

    set_slider(slider, 50)
    wait_for_condition("the lowered volume never applied") { volume_calls == [ 0, 0.5 ] }
    assert_empty audio_context_calls
  end

  test "boosting one participant mutes their element until the slider returns" do
    room = rooms(:designers)
    david = users(:david)
    mapping = [ { id: david.id, name: "David", avatar_url: "", identities: [ "remote-1" ] } ]

    visit room_path(room)
    wait_for_cable_connection
    pin_participant_mapping(room.id, mapping)
    install_stub_room(room.id)
    page.execute_script(<<~JS)
      window.__remoteMic.track.attachedElements = [ window.__boostElement = document.createElement("audio") ];
    JS
    dispatch_participant_mapping(room.id, mapping)

    # The gain carries boosted audio, so the element goes quiet while the
    # boost stands; otherwise it would play twice, and 0% would not silence.
    slider = find("li[data-participant-identity='remote-1'] .huddle__participant-volume")
    set_slider(slider, 150)

    wait_for_condition("the boost never engaged") { audio_context_calls == [ "set" ] }
    assert_equal true, page.evaluate_script("window.__boostElement.muted")
    assert_equal 0, page.evaluate_script("window.__boostElement.volume")

    set_slider(slider, 100)
    wait_for_condition("the boost never disengaged") { audio_context_calls == [ "set", "unset" ] }
    assert_equal false, page.evaluate_script("window.__boostElement.muted")
  end

  test "a boost applied before the track arrives still engages on subscribe" do
    room = rooms(:designers)
    david = users(:david)
    mapping = [ { id: david.id, name: "David", avatar_url: "", identities: [ "remote-1" ] } ]

    visit room_path(room)
    wait_for_cable_connection
    pin_participant_mapping(room.id, mapping)
    join_with_hanging_connect(room)
    page.execute_script("window.localStorage.setItem('campfire.huddle.volume.#{david.id}', '150')")
    dispatch_participant_mapping(room.id, mapping)

    page.execute_script(<<~JS)
      const controller = window.__huddleController;
      window.__volumeCalls = [];
      window.__audioContextCalls = [];
      let publication = { isSubscribed: true, track: null };
      const remote = {
        identity: "remote-1", name: "David", isSpeaking: false,
        trackPublications: new Map(), setVolume: (volume) => window.__volumeCalls.push(volume),
        getTrackPublication: () => publication
      };
      controller.room.remoteParticipants.set("remote-1", remote);
      const audioStub = () => ({ kind: "audio", attach: () => document.createElement("audio") });
      controller.room.emit(window.__livekit.RoomEvent.TrackSubscribed, audioStub(), publication, remote);
      publication = {
        isSubscribed: true,
        track: { setAudioContext: (context) => window.__audioContextCalls.push(context ? "set" : "unset") }
      };
      controller.room.emit(window.__livekit.RoomEvent.TrackSubscribed, audioStub(), publication, remote);
    JS

    wait_for_condition("the late track never engaged the gain") { audio_context_calls == [ "set" ] }
    assert_equal [ 1.5, 1.5 ], volume_calls
  end

  test "a suspended boost context resumes from the resume-audio control" do
    room = rooms(:designers)
    david = users(:david)
    mapping = [ { id: david.id, name: "David", avatar_url: "", identities: [ "remote-1" ] } ]

    visit room_path(room)
    wait_for_cable_connection
    pin_participant_mapping(room.id, mapping)
    install_stub_room(room.id)
    dispatch_participant_mapping(room.id, mapping)
    page.execute_script(<<~JS)
      window.__huddleController.room.canPlaybackAudio = true;
      window.__huddleController.room.startAudio = () => Promise.resolve();
    JS

    slider = find("li[data-participant-identity='remote-1'] .huddle__participant-volume")
    set_slider(slider, 150)
    wait_for_condition("the boost never engaged") { audio_context_calls == [ "set" ] }

    # A remembered boost engages outside any gesture, so its context may
    # sit suspended: force that state, then re-apply with resume stubbed
    # out, the way a gestureless Safari behaves.
    page.execute_script("window.__huddleController.remoteAudioContext.suspend()")
    wait_for_condition("the boost context never suspended") do
      page.evaluate_script("window.__huddleController.remoteAudioContext.state") == "suspended"
    end
    page.execute_script(<<~JS)
      window.__huddleController.remoteAudioContext.resume = () => Promise.resolve();
    JS
    set_slider(slider, 150)

    assert_selector "[data-huddle-target='resumeAudio']:not([hidden])", text: "Play huddle audio"

    page.execute_script(<<~JS)
      window.__resumeCalls = 0;
      const context = window.__huddleController.remoteAudioContext;
      delete context.resume;
      const resume = context.resume.bind(context);
      context.resume = () => {
        window.__resumeCalls += 1;
        return resume();
      };
    JS
    click_button "Play huddle audio"

    wait_for_condition("the resume control never resumed the boost context") do
      page.evaluate_script("window.__huddleController.remoteAudioContext.state") == "running"
    end
    assert_equal 1, page.evaluate_script("window.__resumeCalls")
    assert_selector "[data-huddle-target='resumeAudio'][hidden]", visible: :all
  end

  test "a resubscribed track re-engages the boost on the new track" do
    room = rooms(:designers)
    david = users(:david)
    mapping = [ { id: david.id, name: "David", avatar_url: "", identities: [ "remote-1" ] } ]

    visit room_path(room)
    wait_for_cable_connection
    pin_participant_mapping(room.id, mapping)
    join_with_hanging_connect(room)
    page.execute_script("window.localStorage.setItem('campfire.huddle.volume.#{david.id}', '150')")
    dispatch_participant_mapping(room.id, mapping)

    page.execute_script(<<~JS)
      const controller = window.__huddleController;
      window.__volumeCalls = [];
      window.__audioContextCalls = [];
      const publication = {
        isSubscribed: true,
        track: { setAudioContext: (context) => window.__audioContextCalls.push(context ? "set" : "unset") }
      };
      const remote = {
        identity: "remote-1", name: "David", isSpeaking: false,
        trackPublications: new Map(), setVolume: (volume) => window.__volumeCalls.push(volume),
        getTrackPublication: () => publication
      };
      controller.room.remoteParticipants.set("remote-1", remote);
      const audioStub = () => ({ kind: "audio", attach: () => document.createElement("audio") });
      const first = audioStub();
      controller.room.emit(window.__livekit.RoomEvent.TrackSubscribed, first, publication, remote);
      controller.room.emit(window.__livekit.RoomEvent.TrackUnsubscribed, first, publication, remote);
      controller.room.emit(window.__livekit.RoomEvent.TrackSubscribed, audioStub(), publication, remote);
    JS

    wait_for_condition("the resubscribed track never re-engaged the gain") do
      audio_context_calls == [ "set", "set" ]
    end
  end

  test "a failing microphone restart retries, falls back, then says so" do
    room = rooms(:designers)
    visit room_path(room)
    wait_for_cable_connection
    install_stub_room(room.id)
    page.execute_script(<<~JS)
      const controller = window.__huddleController;
      controller.noiseSuppressionAvailable = true;
      controller.noiseSuppressionEnabled = true;
      window.__processor = { name: "test-processor" };
      window.__audioTrack.mediaStreamTrack.readyState = "ended";
      window.__audioTrack._constraints = { noiseSuppression: false, testMarker: "stored" };
      window.__restartTrackImpl = (...args) => {
        window.__restartCalls.push(args);
        return Promise.reject(new Error("restart failed"));
      };
    JS

    page.execute_script("window.__huddleController.toggleNoiseSuppression()")

    assert_selector "[data-huddle-target='notice']:not([hidden])", text: "couldn’t be restarted"
    assert_equal 3, restart_calls.length
    assert_nil restart_calls[0][0]["testMarker"]
    assert_equal "stored", restart_calls[2][0]["testMarker"]

    # A later success clears the error without touching anything else.
    page.execute_script(<<~JS)
      window.__audioTrack.mediaStreamTrack.readyState = "live";
      window.__restartTrackImpl = (...args) => {
        window.__restartCalls.push(args);
        return Promise.resolve();
      };
      window.__huddleController.toggleNoiseSuppression();
    JS

    assert_selector "[data-huddle-target='notice'][hidden]", visible: :all, wait: 10
  end

  test "switching suppression off re-acquires before stopping the processor" do
    room = rooms(:designers)
    visit room_path(room)
    wait_for_cable_connection
    install_stub_room(room.id)
    page.execute_script(<<~JS)
      window.__huddleController.noiseSuppressionAvailable = true;
      window.__huddleController.noiseSuppressionEnabled = true;
      window.__processor = { name: "test-processor" };
    JS

    page.execute_script("window.__huddleController.toggleNoiseSuppression()")

    wait_for_condition("the processor never stopped") do
      page.evaluate_script("window.__noiseOrder") == [ "restart", "stop" ]
    end
    assert_equal 1, restart_calls.length
  end

  test "an ended microphone restores suppression once the SDK restarts it" do
    room = rooms(:designers)
    visit room_path(room)
    wait_for_cable_connection
    install_stub_room(room.id)
    page.execute_script(<<~JS)
      window.__huddleController.noiseSuppressionAvailable = true;
      window.__huddleController.noiseSuppressionEnabled = true;
      window.__processor = { name: "test-processor" };
      window.__audioTrack._constraints = { noiseSuppression: true };
    JS

    # Mute and unmute so the meter — and the track-ended listener behind the
    # restore — attaches to the microphone track.
    find("[data-huddle-target='mute']").click
    wait_for_condition("muting did not reach the room") { mic_calls.length >= 1 }
    find("[data-huddle-target='mute']").click
    wait_for_condition("unmuting did not reach the room") { mic_calls.length >= 2 }
    wait_for_condition("the meter never attached its ended listener") do
      page.evaluate_script("typeof window.__trackHandlers['ended'] === 'function'")
    end
    assert_empty processor_calls

    # The unplug lost the processor; the SDK's restart ends the old track.
    page.execute_script(<<~JS)
      window.__processor = null;
      window.__trackHandlers["ended"]();
    JS

    wait_for_condition("suppression was never restored") { processor_calls.length >= 1 }
    assert_not_equal "stop", processor_calls.first
  end

  test "an SDK device retarget restores suppression onto the new track" do
    room = rooms(:designers)
    visit room_path(room)
    wait_for_cable_connection
    join_with_hanging_connect(room)

    # The retargeted microphone lost its processor; the sync re-attaches it
    # while the stored constraints already match, so nothing re-acquires.
    page.execute_script(<<~JS)
      const controller = window.__huddleController;
      controller.noiseSuppressionAvailable = true;
      controller.noiseSuppressionEnabled = true;
      window.__processorCalls = [];
      const audioTrack = {
        _constraints: { noiseSuppression: false },
        get constraints() { return this._constraints; },
        setProcessor: (processor) => {
          window.__processorCalls.push(processor);
          return Promise.resolve();
        },
        getProcessor: () => null
      };
      controller.room.localParticipant.getTrackPublication = () => ({ audioTrack });
      controller.room.emit(window.__livekit.RoomEvent.ActiveDeviceChanged, "audioinput");
    JS

    wait_for_condition("suppression was never restored onto the retargeted track") do
      processor_calls.length >= 1
    end
    assert_selector "#channel-huddle[data-state='connected']"
  end

  test "raised hands notify the host in queue order" do
    room = create_call_room(Rooms::Stage, name: "Town Hall", members: [ users(:david), users(:jason), users(:kevin) ])

    using_session("Host") do
      @sessions << "Host"
      sign_in "david@37signals.com"
      visit room_path(room)
      wait_for_cable_connection
      find("button[aria-label='Show stage']").click
      assert_selector ".stage-panel__surface h2", text: "Stage"
      page.execute_script(<<~JS)
        window.__handEvents = [];
        window.addEventListener("stage:hands-changed", (event) => window.__handEvents.push(event.detail));
      JS
    end

    visit room_path(room)
    wait_for_cable_connection
    find("button[aria-label='Show stage']").click
    click_button "Raise hand"
    assert_selector "##{dom_id(room, :stage_controls)}", text: "Lower hand"

    using_session("Kevin") do
      @sessions << "Kevin"
      sign_in "kevin@37signals.com"
      visit room_path(room)
      wait_for_cable_connection
      find("button[aria-label='Show stage']").click
      click_button "Raise hand"
    end

    using_session("Host") do
      jason_row = "##{dom_id(room.memberships.find_by!(user: users(:jason)), :stage_row)}"
      kevin_row = "##{dom_id(room.memberships.find_by!(user: users(:kevin)), :stage_row)}"

      within jason_row, wait: BROADCAST_WAIT do
        assert_selector ".stage-panel__hand-badge", text: "#1 in queue", wait: BROADCAST_WAIT
      end
      within kevin_row do
        assert_selector ".stage-panel__hand-badge", text: "#2 in queue", wait: BROADCAST_WAIT
      end

      first, second = page.evaluate_script(<<~JS)
        Array.from(document.querySelectorAll("[aria-label='Listeners'] .stage-panel__member"))
          .map((element) => element.id)
      JS
      assert_equal [ jason_row.delete_prefix("#"), kevin_row.delete_prefix("#") ], [ first, second ]

      assert_selector "[data-stage-panel-target='handAnnouncement']",
        text: "2 listeners have their hands raised.", visible: :all, wait: BROADCAST_WAIT
      assert_equal [ { "count" => 1 }, { "count" => 2 } ], page.evaluate_script("window.__handEvents")
    end

    using_session("Kevin") do
      assert_equal "", page.evaluate_script(
        "document.querySelector(\"[data-stage-panel-target='handAnnouncement']\").textContent")
    end
  end

  test "host hand-raise chimes debounce per membership" do
    room = create_call_room(Rooms::Stage, name: "Town Hall", members: [ users(:david), users(:jason), users(:kevin) ])
    jason_row = "##{dom_id(room.memberships.find_by!(user: users(:jason)), :stage_row)}"
    kevin_row = "##{dom_id(room.memberships.find_by!(user: users(:kevin)), :stage_row)}"

    sign_in "david@37signals.com"
    visit room_path(room)
    wait_for_cable_connection
    find("button[aria-label='Show stage']").click
    assert_selector "#{jason_row} .stage-panel__role"

    page.execute_script(<<~JS)
      window.__handEvents = [];
      window.addEventListener("stage:hands-changed", (event) => window.__handEvents.push(event.detail.count));
      window.__chimeOscillators = 0;
      window.__RealAudioContext = window.AudioContext;
      window.AudioContext = class {
        constructor() { this.state = "running"; this.currentTime = 0; this.destination = {}; }
        resume() { return Promise.resolve(); }
        createOscillator() {
          window.__chimeOscillators += 1;
          return { type: "", frequency: {}, connect() {}, start() {}, stop() {} };
        }
        createGain() {
          return { gain: { setValueAtTime() {}, linearRampToValueAtTime() {} }, connect() {} };
        }
      };
      window.__now = Date.now();
      window.__RealDateNow = Date.now;
      Date.now = () => window.__now;
      window.__setHandRaised = (row, raised) => {
        const role = document.querySelector(`${row} .stage-panel__role`);
        role.querySelector(".stage-panel__hand-badge")?.remove();
        if (raised) {
          const badge = document.createElement("span");
          badge.className = "stage-panel__hand-badge";
          badge.textContent = "Hand raised";
          role.appendChild(badge);
        }
      };
    JS

    # A first raise chimes and announces.
    page.execute_script("window.__setHandRaised('#{jason_row}', true)")
    wait_for_condition("the first raise never chimed") { page.evaluate_script("window.__chimeOscillators") == 2 }
    assert_selector "[data-stage-panel-target='handAnnouncement']", text: "A listener raised their hand.", visible: :all

    # Lowering and re-raising within a minute stays silent, while every
    # roster change still reports the queue itself.
    page.execute_script("window.__setHandRaised('#{jason_row}', false)")
    wait_for_condition("the lower never registered") { page.evaluate_script("window.__handEvents") == [ 1, 0 ] }
    page.execute_script("window.__setHandRaised('#{jason_row}', true)")
    sleep 0.5
    assert_equal 2, page.evaluate_script("window.__chimeOscillators")
    assert_equal [ 1, 0, 1 ], page.evaluate_script("window.__handEvents")

    # Another member's raise still chimes.
    page.execute_script("window.__setHandRaised('#{kevin_row}', true)")
    wait_for_condition("the second member never chimed") { page.evaluate_script("window.__chimeOscillators") == 4 }
    assert_selector "[data-stage-panel-target='handAnnouncement']", text: "2 listeners have their hands raised.", visible: :all

    # After a minute the first member chimes again.
    page.execute_script(<<~JS)
      window.__now += 61_000;
      window.__setHandRaised('#{jason_row}', false);
    JS
    wait_for_condition("the second lower never registered") { page.evaluate_script("window.__handEvents") == [ 1, 0, 1, 2, 1 ] }
    page.execute_script("window.__setHandRaised('#{jason_row}', true)")
    wait_for_condition("the re-raise after a minute never chimed") { page.evaluate_script("window.__chimeOscillators") == 6 }

    page.execute_script(<<~JS)
      window.AudioContext = window.__RealAudioContext;
      Date.now = window.__RealDateNow;
    JS
  end

  test "a host mutes and unmutes a speaker from the stage panel" do
    room = create_call_room(Rooms::Stage, name: "Town Hall", members: [ users(:david), users(:jason) ])
    speaker = room.memberships.find_by!(user: users(:jason))
    speaker.change_stage_role!("speaker")

    sign_in "david@37signals.com"
    visit room_path(room)
    wait_for_cable_connection
    find("button[aria-label='Show stage']").click

    # Your own row keeps the role controls but no moderation forms: the
    # server rejects moderating your own session.
    david_row = "##{dom_id(room.memberships.find_by!(user: users(:david)), :stage_row)}"
    within david_row do
      assert_selector "form[action*='stage/roles']"
      assert_no_selector "form[action*='call_moderation']"
    end

    jason_row = "##{dom_id(speaker, :stage_row)}"
    within jason_row do
      click_button "Mute"
    end
    within jason_row do
      assert_selector ".stage-panel__muted-badge", text: "Muted", wait: BROADCAST_WAIT
      click_button "Unmute"
      assert_no_selector ".stage-panel__muted-badge", wait: BROADCAST_WAIT
    end
    assert_not speaker.reload.server_muted?
  end

  test "saving call settings updates the permanent panel without a reload" do
    sign_in "david@37signals.com"
    visit user_profile_url

    assert_selector "#channel-huddle[data-huddle-voice-mode-value='voice_activity']", visible: :all

    select "Push to talk", from: "Microphone mode"
    fill_in "Push-to-talk key", with: "CapsLock"
    page.execute_script("window.__noReload = true")
    within(:xpath, "//fieldset[legend/text()='Calls']/ancestor::form") do
      find("button[type='submit']").click
    end

    assert_selector "#channel-huddle[data-huddle-voice-mode-value='push_to_talk']", visible: :all, wait: 10
    assert_selector "#channel-huddle[data-huddle-push-to-talk-key-value='CapsLock']", visible: :all
    assert_equal true, page.evaluate_script("window.__noReload")
  end

  private
    def create_call_room(type, name:, members:)
      type.create_for({ name:, creator: members.first }, users: members).tap do |room|
        @created_rooms << room
      end
    end

    def hold_push_to_talk(key)
      page.execute_script(<<~JS, key)
        document.body.dispatchEvent(new KeyboardEvent("keydown", { key: arguments[0], bubbles: true }));
      JS
    end

    def release_push_to_talk(key)
      page.execute_script(<<~JS, key)
        document.body.dispatchEvent(new KeyboardEvent("keyup", { key: arguments[0], bubbles: true }));
      JS
    end

    def set_slider(slider, value)
      page.execute_script(<<~JS, slider, value)
        arguments[0].value = arguments[1];
        arguments[0].dispatchEvent(new Event("input", { bubbles: true }));
      JS
    end

    def mic_calls
      page.evaluate_script("window.__micCalls") || []
    end

    def volume_calls
      page.evaluate_script("window.__volumeCalls") || []
    end

    def subscribe_calls
      page.evaluate_script("window.__subscribeCalls") || []
    end

    def audio_context_calls
      page.evaluate_script("window.__audioContextCalls") || []
    end

    def restart_calls
      page.evaluate_script("window.__restartCalls") || []
    end

    def processor_calls
      page.evaluate_script("window.__processorCalls") || []
    end

    def participant_volume(user_id)
      page.evaluate_script("window.localStorage.getItem('campfire.huddle.volume.#{user_id}')")
    end

    def reconnect_countdown
      page.evaluate_script("document.querySelector(\"[data-huddle-target='reconnectCountdown']\").textContent")
    end

    def wait_for_condition(message, timeout: 10)
      deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + timeout
      until yield
        flunk message if Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline
        sleep 0.05
      end
      assert true
    end

    # The presence stacks feed the identity mapping through this event; the
    # test pins both JSON sources so no 15-second poll disagrees mid-test.
    def pin_participant_mapping(room_id, participants)
      page.execute_script(<<~JS, room_id, participants.to_json)
        const pinned = JSON.parse(arguments[1]);
        const roomId = arguments[0];
        if (!window.__nativeFetch) window.__nativeFetch = window.fetch.bind(window);
        window.fetch = (...args) => {
          const input = args[0];
          const url = new URL(typeof input === "string" ? input : input.url, window.location.origin);
          if (url.pathname === "/users/huddle_presence") {
            return Promise.resolve(new Response(
              JSON.stringify([ { room_id: roomId, participants: pinned } ]),
              { status: 200, headers: { "Content-Type": "application/json" } }));
          }
          if (url.pathname.endsWith("/huddle/participants")) {
            return Promise.resolve(new Response(JSON.stringify(pinned),
              { status: 200, headers: { "Content-Type": "application/json" } }));
          }
          return window.__nativeFetch(...args);
        };
      JS

      # The sidebar's aggregate poll fired on page load with the native fetch,
      # before this stub existed. Its real response — nobody in the call —
      # must land before the synthetic mapping the test dispatches next; on a
      # slow runner it arrives after it instead and wipes the mapping, hiding
      # the per-participant controls the test is about to find. Later polls
      # use the stub above and reaffirm the pinned mapping.
      wait_for_condition("the sidebar presence poll never settled") do
        page.evaluate_script(<<~JS)
          ([ ...document.querySelectorAll('[data-controller~="huddle-presence"]') ])
            .every((element) => {
              const controller = window.Stimulus?.getControllerForElementAndIdentifier(element, "huddle-presence");
              return !controller || !controller.inFlightRefresh;
            })
        JS
      end
    end

    def dispatch_participant_mapping(room_id, participants)
      page.execute_script(<<~JS, room_id, participants.to_json)
        window.dispatchEvent(new CustomEvent("huddle-participants:updated", {
          detail: { url: `/rooms/${arguments[0]}/huddle/participants`, participants: JSON.parse(arguments[1]) }
        }));
      JS
    end

    # Binds the real controller to a real LiveKit room whose signaling
    # socket never opens: the full event surface works with no server behind
    # it. The constructor patch only affects sockets created after it, so the
    # already-connected ActionCable socket is untouched, and the global is
    # restored once the room holds its hanging socket. The listener hint
    # skips the prejoin check; the real token still decides publishing after
    # the bind.
    def join_with_hanging_connect(room)
      page.execute_script(<<~JS, room.id)
        (async (roomId) => {
          const controller = window.Stimulus
            .getControllerForElementAndIdentifier(document.getElementById("channel-huddle"), "huddle");
          window.__huddleController = controller;
          const livekit = await import("livekit-client");
          window.__livekit = livekit;
          window.__nativeWebSocket = window.WebSocket;
          window.WebSocket = class HangingTestSocket {
            constructor(url) {
              this.url = url;
              this.readyState = 0;
            }
            close() {}
            send() {}
            addEventListener() {}
            removeEventListener() {}
          };
          const csrfMeta = document.createElement("meta");
          csrfMeta.name = "csrf-token";
          csrfMeta.content = "test-csrf-token";
          document.head.appendChild(csrfMeta);
          window.dispatchEvent(new CustomEvent("huddle:join", {
            detail: { roomId, roomName: "Test", canPublishHint: false }
          }));
        })(arguments[0]);
      JS
      wait_for_condition("the room never bound") do
        page.evaluate_script("window.__huddleController?.room != null")
      end
      page.execute_script(<<~JS)
        window.WebSocket = window.__nativeWebSocket;
        // The socket never opens, so drive the state through the same event
        // a real recovery would deliver.
        window.__huddleController.room.emit(window.__livekit.RoomEvent.Reconnected);
      JS
      wait_for_condition("the panel never connected") do
        page.evaluate_script("window.__huddleController?.state") == "connected"
      end
    end

    # A connected panel with a stubbed room: a local and a remote
    # participant, a mic publication carrying a stub audio track with
    # processor and restart hooks, and a remote microphone with volume and
    # subscription hooks.
    def install_stub_room(room_id)
      page.execute_script(<<~JS, room_id)
        const controller = window.Stimulus
          .getControllerForElementAndIdentifier(document.getElementById("channel-huddle"), "huddle");
        window.__huddleController = controller;
        controller.liveKit = {
          Track: { Source: { Microphone: "microphone" }, Kind: { Audio: "audio", Video: "video" } },
          TrackEvent: { Ended: "ended" },
          createAudioAnalyser: () => ({ calculateVolume: () => 0.4, cleanup: () => {} })
        };
        controller.roomId = arguments[0];
        controller.state = "connected";
        controller.canPublish = true;
        controller.noiseSuppressionAvailable = false;
        controller.noiseSuppressionEnabled = false;
        window.__micCalls = [];
        window.__processorCalls = [];
        window.__restartCalls = [];
        window.__noiseOrder = [];
        window.__volumeCalls = [];
        window.__subscribeCalls = [];
        window.__audioContextCalls = [];
        window.__trackHandlers = {};
        window.__processor = null;
        let micEnabled = true;
        const mediaStreamTrack = { readyState: "live" };
        const audioTrack = {
          mediaStreamTrack,
          _constraints: { noiseSuppression: false },
          // The SDK's constraints getter reads the stored copy, so a sync's
          // write is visible here the same way.
          get constraints() { return this._constraints; },
          isMuted: false,
          setProcessor: (processor) => {
            window.__processorCalls.push(processor);
            window.__noiseOrder.push("setProcessor");
            window.__processor = processor;
            return Promise.resolve();
          },
          getProcessor: () => window.__processor,
          stopProcessor: () => {
            window.__processorCalls.push("stop");
            window.__noiseOrder.push("stop");
            window.__processor = null;
            return Promise.resolve();
          },
          restartTrack: (...args) => window.__restartTrackImpl(...args),
          on: (event, handler) => { window.__trackHandlers[event] = handler; }
        };
        window.__audioTrack = audioTrack;
        window.__restartTrackImpl = (...args) => {
          window.__restartCalls.push(args);
          window.__noiseOrder.push("restart");
          return Promise.resolve();
        };
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
          trackPublications: new Map(),
          getTrackPublication: (source) => source === "microphone" ? { audioTrack } : null
        };
        const remoteAudioTrack = {
          setAudioContext: (context) => { window.__audioContextCalls.push(context ? "set" : "unset"); }
        };
        const remoteMic = {
          isMuted: false,
          isSubscribed: true,
          track: remoteAudioTrack,
          setSubscribed: (subscribed) => {
            window.__subscribeCalls.push(subscribed);
            remoteMic.isSubscribed = subscribed;
          }
        };
        const remote = {
          identity: "remote-1",
          name: "David",
          isSpeaking: false,
          trackPublications: new Map(),
          setVolume: (volume) => { window.__volumeCalls.push(volume); },
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
end
