require "application_system_test_case"
require "timeout"

class HuddleJoinNoticesTest < ApplicationSystemTestCase
  setup do
    @environment_names = Huddle::REQUIRED_ENVIRONMENT
    @original_livekit_environment = ENV.values_at(*@environment_names)
    ENV["LIVEKIT_URL"] = "wss://huddle.example.test"
    ENV["LIVEKIT_INTERNAL_URL"] = "ws://livekit.example.test:7880"
    ENV["LIVEKIT_API_KEY"] = "test-api-key"
    ENV["LIVEKIT_API_SECRET"] = "test-api-secret"
    ENV["LIVEKIT_GATEWAY_SECRET"] = "test-gateway-secret"
    Rails.configuration.x.web_push_pool.stubs(:queue)
    sign_in "jason@37signals.com"
  end

  teardown do
    @environment_names.zip(@original_livekit_environment).each { |name, value| ENV[name] = value }
  end

  test "an in-call member sees a join toast and hears the join sound" do
    room = rooms(:david_and_jason)
    visit room_path(room)
    wait_for_cable_connection
    wait_for_join_notice_controller
    hold_toasts_open
    record_played_sounds

    jason_grant = HuddleGrant.issue!(session: second_session_for(users(:jason)),
      membership: memberships(:jason_david_and_jason))
    jason_grant.update_columns(last_seen_at: Time.current)
    mark_in_call(room)

    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))
    notify_join_and_deliver(grant)

    assert_selector ".huddle-join-toast:not(.huddle-join-toast--leave)", text: "David joined", wait: 10
    wait_for_condition("the join sound never played") { played_sounds.any? }
    assert_equal 1, played_sounds.size
    assert_includes played_sounds.first, "incoming"
  end

  test "rapid joins batch into one toast with one sound" do
    group = Rooms::Direct.create_for({ creator: users(:david) }, users: [ users(:david), users(:jason), users(:kevin) ])
    visit room_path(group)
    wait_for_cable_connection
    wait_for_join_notice_controller
    hold_toasts_open(batch_window: 30_000)
    record_played_sounds

    jason_grant = HuddleGrant.issue!(session: second_session_for(users(:jason)),
      membership: group.memberships.find_by!(user: users(:jason)))
    jason_grant.update_columns(last_seen_at: Time.current)
    mark_in_call(group)

    david_grant = HuddleGrant.issue!(session: sessions(:david_safari),
      membership: group.memberships.find_by!(user: users(:david)))
    kevin_grant = HuddleGrant.issue!(session: second_session_for(users(:kevin)),
      membership: group.memberships.find_by!(user: users(:kevin)))
    # Asserted between notifies: the two broadcasts can arrive in either
    # order, so the batch asserts one name at a time (the 30s batch window
    # holds the toast open across the round trip).
    notify_join_and_deliver(david_grant)
    assert_selector ".huddle-join-toast:not(.huddle-join-toast--leave)", text: "David joined", wait: 10
    notify_join_and_deliver(kevin_grant)

    assert_selector ".huddle-join-toast:not(.huddle-join-toast--leave)", text: "David and Kevin joined", wait: 10
    wait_for_condition("the join sound never played") { played_sounds.any? }
    assert_equal 1, played_sounds.size
  end

  test "a join toast stays silent with DND on" do
    room = rooms(:david_and_jason)
    visit room_path(room)
    wait_for_cable_connection
    wait_for_join_notice_controller
    hold_toasts_open
    record_played_sounds
    page.execute_script(<<~JS)
      const meta = document.createElement("meta")
      meta.name = "notification-dnd"
      meta.content = "muted"
      document.head.append(meta)
    JS

    jason_grant = HuddleGrant.issue!(session: second_session_for(users(:jason)),
      membership: memberships(:jason_david_and_jason))
    jason_grant.update_columns(last_seen_at: Time.current)
    mark_in_call(room)

    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))
    notify_join_and_deliver(grant)

    assert_selector ".huddle-join-toast:not(.huddle-join-toast--leave)", text: "David joined", wait: 10
    sleep 0.5
    assert_empty played_sounds
  end

  test "join and leave toasts announce through the container live region" do
    room = rooms(:david_and_jason)
    visit room_path(room)
    wait_for_cable_connection
    wait_for_join_notice_controller
    hold_toasts_open(leave_delay: 200)

    assert_selector '.huddle-join-toasts[aria-live="polite"]', visible: :all

    jason_grant = HuddleGrant.issue!(session: second_session_for(users(:jason)),
      membership: memberships(:jason_david_and_jason))
    jason_grant.update_columns(last_seen_at: Time.current)
    mark_in_call(room)

    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))
    notify_join_and_deliver(grant)
    assert_selector ".huddle-join-toast:not(.huddle-join-toast--leave)", text: "David joined", wait: 10

    assert grant.mark_out_of_call!
    assert_selector ".huddle-join-toast--leave", text: "David left", wait: 10

    assert_no_selector ".huddle-join-toast[role]"
    assert_no_selector ".huddle-join-toast[aria-live]"
  end

  test "a leave toasts quietly in the call without the join sound" do
    room = rooms(:david_and_jason)
    visit room_path(room)
    wait_for_cable_connection
    wait_for_join_notice_controller
    hold_toasts_open(leave_delay: 200)
    record_played_sounds

    jason_grant = HuddleGrant.issue!(session: second_session_for(users(:jason)),
      membership: memberships(:jason_david_and_jason))
    jason_grant.update_columns(last_seen_at: Time.current)
    mark_in_call(room)

    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))
    grant.update_columns(last_seen_at: Time.current)
    assert grant.mark_out_of_call!

    assert_selector ".huddle-join-toast--leave", text: "David left", wait: 10
    sleep 0.5
    assert_empty played_sounds
  end

  test "a server mute cycle toasts neither left nor joined" do
    room = rooms(:david_and_jason)
    visit room_path(room)
    wait_for_cable_connection
    wait_for_join_notice_controller
    # Production waits 5s; 1.5s keeps the test quick while leaving CI room
    # for the rejoin broadcast to land inside the window.
    hold_toasts_open(leave_delay: 1500)
    record_played_sounds

    jason_grant = HuddleGrant.issue!(session: second_session_for(users(:jason)),
      membership: memberships(:jason_david_and_jason))
    jason_grant.update_columns(last_seen_at: Time.current)
    mark_in_call(room)

    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))
    grant.update_columns(last_seen_at: Time.current)

    # A server mute revokes the grant (leave fires) and the client rejoins
    # with a fresh grant (join fires) inside the leave delay.
    grant.revoke!
    rejoined = HuddleGrant.issue!(session: sessions(:david_safari),
      membership: memberships(:david_david_and_jason))
    notify_join_and_deliver(rejoined)

    sleep 2 # past the 1.5s leave delay
    assert_no_selector ".huddle-join-toast"
    assert_empty played_sounds
  end

  test "a server mute cycle across a room switch stays silent" do
    group = Rooms::Direct.create_for({ creator: users(:david) }, users: [ users(:david), users(:jason), users(:kevin) ])
    visit room_path(group)
    wait_for_cable_connection
    wait_for_join_notice_controller
    hold_toasts_open(leave_delay: 30_000)
    record_played_sounds

    jason_grant = HuddleGrant.issue!(session: second_session_for(users(:jason)),
      membership: group.memberships.find_by!(user: users(:jason)))
    jason_grant.update_columns(last_seen_at: Time.current)
    mark_in_call(group)

    david_grant = HuddleGrant.issue!(session: sessions(:david_safari),
      membership: group.memberships.find_by!(user: users(:david)))
    david_grant.update_columns(last_seen_at: Time.current)

    # The leave lands, then the viewer switches rooms mid-delay through a
    # Turbo navigation: the pending leave must survive the Stimulus
    # reconnect and still swallow the rejoin.
    david_grant.revoke!
    sleep 1 # let the leave broadcast arrive before navigating away
    page.execute_script("Turbo.visit(arguments[0])", room_path(rooms(:watercooler)))
    wait_for_condition("navigation never reached the channel") do
      page.evaluate_script("window.location.pathname") == room_path(rooms(:watercooler))
    end
    wait_for_cable_connection
    wait_for_join_notice_controller
    wait_for_join_notice_subscription
    sleep 1 # let the resubscribe confirm before notifying
    # The reconnect re-queries the panel, which reports idle without
    # LiveKit; the viewer is still in the group call, so say so again.
    mark_in_call(group)

    rejoined = HuddleGrant.issue!(session: sessions(:david_safari),
      membership: group.memberships.find_by!(user: users(:david)))
    notify_join_and_deliver(rejoined)

    # A fresh join still toasts: notices are flowing, the mute cycle
    # alone stayed silent.
    kevin_grant = HuddleGrant.issue!(session: second_session_for(users(:kevin)),
      membership: group.memberships.find_by!(user: users(:kevin)))
    notify_join_and_deliver(kevin_grant)

    assert_selector ".huddle-join-toast", exact_text: "Kevin joined", wait: 10
    assert_no_selector ".huddle-join-toast", text: "David"
    assert_equal 1, played_sounds.size
  end

  test "joining a sidebar pill from another room navigates then joins" do
    room = rooms(:david_and_jason)
    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))
    ActivityItem.where(user: users(:jason), event_type: "huddle_started").update_all(event_type: "huddle_missed")

    visit room_path(rooms(:watercooler))
    wait_for_cable_connection
    wait_for_join_notice_controller
    page.execute_script("window.huddleJoinEvents = []; window.addEventListener('huddle:join', event => window.huddleJoinEvents.push(event.detail))")
    assert_selector "##{dom_id(room, :list)}", wait: 10

    notify_join_and_deliver(grant)

    within "##{dom_id(room, :list)}" do
      assert_selector ".huddle-join-pill", text: "David is in your huddle", wait: 10
      click_button "Join"
    end

    wait_for_condition("navigation never reached the DM") do
      page.evaluate_script("window.location.pathname") == room_path(room)
    end
    wait_for_condition("join was never dispatched") do
      page.evaluate_script("window.huddleJoinEvents.length") > 0
    end
    assert_equal [ { "roomId" => room.id, "roomName" => "David" } ], page.evaluate_script("window.huddleJoinEvents")
    assert_no_selector "#huddle-join-banner-slot .huddle-join-banner", wait: 10
    assert_no_selector ".huddle-join-pill", wait: 10
  end

  test "an out-of-call member sees the banner and sidebar pill and joins from the banner" do
    room = rooms(:david_and_jason)
    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))
    ActivityItem.where(user: users(:jason), event_type: "huddle_started").update_all(event_type: "huddle_missed")

    visit room_path(room)
    wait_for_cable_connection
    wait_for_join_notice_controller
    page.execute_script("window.huddleJoinEvents = []; window.addEventListener('huddle:join', event => window.huddleJoinEvents.push(event.detail))")
    assert_selector "##{dom_id(room, :list)}", wait: 10

    notify_join_and_deliver(grant)

    assert_selector "#huddle-join-banner-slot .huddle-join-banner", text: "David is in your huddle", wait: 10
    within "##{dom_id(room, :list)}" do
      assert_selector ".huddle-join-pill", text: "David is in your huddle", wait: 10
    end

    within "#huddle-join-banner-slot" do
      click_button "Join"
    end

    wait_for_condition("join was never dispatched") do
      page.evaluate_script("window.huddleJoinEvents.length") > 0
    end
    assert_equal [ { "roomId" => room.id, "roomName" => "David" } ], page.evaluate_script("window.huddleJoinEvents")
    assert_no_selector "#huddle-join-banner-slot .huddle-join-banner", wait: 10
    assert_no_selector ".huddle-join-pill", wait: 10
  end

  test "the join banner drops each leaver and hides when empty" do
    group = Rooms::Direct.create_for({ creator: users(:david) }, users: [ users(:david), users(:jason), users(:kevin) ])
    david_grant = HuddleGrant.issue!(session: sessions(:david_safari),
      membership: group.memberships.find_by!(user: users(:david)))
    kevin_grant = HuddleGrant.issue!(session: second_session_for(users(:kevin)),
      membership: group.memberships.find_by!(user: users(:kevin)))
    ActivityItem.where(user: users(:jason), event_type: "huddle_started").update_all(event_type: "huddle_missed")

    visit room_path(group)
    wait_for_cable_connection
    wait_for_join_notice_controller
    assert_selector "##{dom_id(group, :list)}", wait: 10

    # Asserted between notifies: the two broadcasts can arrive in either
    # order, so the roster asserts one name at a time.
    notify_join_and_deliver(david_grant)
    assert_selector "#huddle-join-banner-slot .huddle-join-banner", text: "David is in your huddle", wait: 10
    notify_join_and_deliver(kevin_grant)
    assert_selector "#huddle-join-banner-slot .huddle-join-banner", text: "David and Kevin are in your huddle", wait: 10

    assert kevin_grant.mark_out_of_call!
    assert_selector "#huddle-join-banner-slot .huddle-join-banner", text: "David is in your huddle", wait: 10
    within "##{dom_id(group, :list)}" do
      assert_selector ".huddle-join-pill", text: "David is in your huddle", wait: 10
    end

    assert david_grant.mark_out_of_call!
    assert_no_selector "#huddle-join-banner-slot .huddle-join-banner", wait: 10
    assert_no_selector ".huddle-join-pill", wait: 10
  end

  test "the join banner reconciles its roster with presence refreshes" do
    group = Rooms::Direct.create_for({ creator: users(:david) }, users: [ users(:david), users(:jason), users(:kevin) ])
    david_grant = HuddleGrant.issue!(session: sessions(:david_safari),
      membership: group.memberships.find_by!(user: users(:david)))
    kevin_grant = HuddleGrant.issue!(session: second_session_for(users(:kevin)),
      membership: group.memberships.find_by!(user: users(:kevin)))
    ActivityItem.where(user: users(:jason), event_type: "huddle_started").update_all(event_type: "huddle_missed")

    visit room_path(group)
    wait_for_cable_connection
    wait_for_join_notice_controller
    assert_selector "##{dom_id(group, :list)}", wait: 10

    # Asserted between notifies: the two broadcasts can arrive in either
    # order, so the roster asserts one name at a time.
    notify_join_and_deliver(david_grant)
    assert_selector "#huddle-join-banner-slot .huddle-join-banner", text: "David is in your huddle", wait: 10
    notify_join_and_deliver(kevin_grant)
    assert_selector "#huddle-join-banner-slot .huddle-join-banner", text: "David and Kevin are in your huddle", wait: 10

    participants_url = Rails.application.routes.url_helpers.participants_room_huddle_path(group)
    page.execute_script(<<~JS, participants_url, users(:david).id)
      window.dispatchEvent(new CustomEvent("huddle-participants:updated", {
        detail: { url: arguments[0], participants: [ { id: arguments[1], name: "David" } ] }
      }))
    JS
    assert_selector "#huddle-join-banner-slot .huddle-join-banner", text: "David is in your huddle", wait: 10

    page.execute_script(<<~JS, participants_url)
      window.dispatchEvent(new CustomEvent("huddle-participants:updated", {
        detail: { url: arguments[0], participants: [] }
      }))
    JS
    assert_no_selector "#huddle-join-banner-slot .huddle-join-banner", wait: 10
    assert_no_selector ".huddle-join-pill", wait: 10
  end

  test "the join banner clears when the huddle ends" do
    room = rooms(:david_and_jason)
    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))
    ActivityItem.where(user: users(:jason), event_type: "huddle_started").update_all(event_type: "huddle_missed")

    visit room_path(room)
    wait_for_cable_connection
    wait_for_join_notice_controller
    assert_selector "##{dom_id(room, :list)}", wait: 10

    notify_join_and_deliver(grant)
    assert_selector "#huddle-join-banner-slot .huddle-join-banner", text: "David is in your huddle", wait: 10

    assert grant.mark_out_of_call!

    assert_no_selector "#huddle-join-banner-slot .huddle-join-banner", wait: 10
    assert_no_selector ".huddle-join-pill", wait: 10
  end

  private
    def second_session_for(user)
      Session.create!(user: user, user_agent: "second device", ip_address: "127.0.0.2")
    end

    # record_seen! fans presence and join notices out through jobs; run
    # the join job inline so the browser receives it without a worker.
    def notify_join_and_deliver(grant)
      perform_enqueued_jobs only: Huddle::JoinNoticeJob do
        grant.record_seen!
      end
    end

    # The panel answers huddle:query with its room and state; fake the
    # answer so the viewer reads as in the room's call without LiveKit.
    def mark_in_call(room)
      page.execute_script(<<~JS, room.id)
        window.dispatchEvent(new CustomEvent("huddle:changed", {
          detail: { roomId: arguments[0], state: "connected" }
        }))
      JS
    end

    def wait_for_join_notice_controller
      Timeout.timeout(10) do
        sleep 0.05 until page.evaluate_script(<<~JS)
          (() => {
            const element = document.getElementById("huddle-join-notices")
            return !!(element && window.Stimulus &&
              window.Stimulus.getControllerForElementAndIdentifier(element, "huddle-join-notice"))
          })()
        JS
      end
    end

    # The controller resubscribes on every Turbo navigation; a join
    # broadcast sent before the new subscription exists is lost, so a
    # room-switch test waits for it explicitly.
    def wait_for_join_notice_subscription
      Timeout.timeout(10) do
        sleep 0.05 until page.evaluate_script(<<~JS)
          (() => {
            const element = document.getElementById("huddle-join-notices")
            const controller = element && window.Stimulus &&
              window.Stimulus.getControllerForElementAndIdentifier(element, "huddle-join-notice")
            return !!(controller && controller.subscription)
          })()
        JS
      end
    end

    # Toasts auto-dismiss after seconds; hold them open so assertions
    # never race the timeout. Leaves wait out a mute-cycle delay before
    # toasting; shorten it so leave assertions never wait the full delay.
    def hold_toasts_open(batch_window: 4_000, leave_delay: 5_000)
      page.execute_script(<<~JS, batch_window, leave_delay)
        document.getElementById("huddle-join-notices").setAttribute("data-huddle-join-notice-toast-timeout-value", "60000")
        document.getElementById("huddle-join-notices").setAttribute("data-huddle-join-notice-batch-window-value", String(arguments[0]))
        document.getElementById("huddle-join-notices").setAttribute("data-huddle-join-notice-leave-delay-value", String(arguments[1]))
      JS
    end

    def record_played_sounds
      page.execute_script(<<~JS)
        window.__playedSounds = [];
        window.Audio.prototype.play = function() {
          window.__playedSounds.push(this.src || this.currentSrc);
          return Promise.resolve();
        };
      JS
    end

    def played_sounds
      page.evaluate_script("window.__playedSounds")
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
