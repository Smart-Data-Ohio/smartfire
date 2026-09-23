require "application_system_test_case"
require "timeout"

class HuddlePresenceTest < ApplicationSystemTestCase
  setup do
    @environment_names = Huddle::REQUIRED_ENVIRONMENT
    @original_livekit_environment = ENV.values_at(*@environment_names)
    ENV["LIVEKIT_URL"] = "wss://huddle.example.test"
    ENV["LIVEKIT_INTERNAL_URL"] = "ws://livekit.example.test:7880"
    ENV["LIVEKIT_API_KEY"] = "test-api-key"
    ENV["LIVEKIT_API_SECRET"] = "test-api-secret"
    ENV["LIVEKIT_GATEWAY_SECRET"] = "test-gateway-secret"
    sign_in "jason@37signals.com"

    @room = rooms(:designers)
  end

  teardown do
    @environment_names.zip(@original_livekit_environment).each { |name, value| ENV[name] = value }
  end

  test "the channel sidebar row and header show participants and empty on revoke" do
    visit room_path(@room)
    wait_for_cable_connection

    assert_selector "#shared_rooms .sidebar-item", text: "Designers"
    within "##{dom_id(@room, :list)}" do
      assert_selector ".voice-stack:not(.voice-stack--live)", visible: :all
    end

    observe_turbo_stream_renders
    renders = header_presence_renders
    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: @room.memberships.find_by!(user: users(:david)))
    wait_for_issuance_broadcast(after: renders)
    record_seen_and_deliver(grant)

    within "##{dom_id(@room, :list)}" do
      assert_selector ".voice-stack--live", wait: BROADCAST_WAIT
      assert_selector ".voice-stack__count", text: "1", wait: BROADCAST_WAIT
      assert_selector "img.voice-stack__avatar[data-user-id='#{users(:david).id}']", wait: BROADCAST_WAIT
    end
    assert_equal "1 in huddle: David", find("##{dom_id(@room, :list)} .voice-stack")["aria-label"]
    within ".room-header__actions" do
      assert_selector ".voice-stack--live", wait: BROADCAST_WAIT
      assert_selector ".voice-stack__count", text: "1", wait: BROADCAST_WAIT
      assert_selector "img.voice-stack__avatar[data-user-id='#{users(:david).id}']", wait: BROADCAST_WAIT
    end

    grant.revoke!

    within "##{dom_id(@room, :list)}" do
      assert_no_selector ".voice-stack--live", wait: BROADCAST_WAIT
      assert_no_selector "img.voice-stack__avatar", wait: BROADCAST_WAIT
    end
    within ".room-header__actions" do
      assert_no_selector ".voice-stack--live", wait: BROADCAST_WAIT
    end
  end

  test "the DM sidebar row and header show the peer and empty on revoke" do
    direct_room = rooms(:david_and_jason)
    visit room_path(direct_room)
    wait_for_cable_connection

    observe_turbo_stream_renders
    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: direct_room.memberships.find_by!(user: users(:david)))
    wait_for_issuance_broadcast(room: direct_room)
    record_seen_and_deliver(grant)

    within "##{dom_id(direct_room, :list)}" do
      assert_selector ".voice-stack--live", wait: BROADCAST_WAIT
      assert_selector ".voice-stack__count", text: "1", wait: BROADCAST_WAIT
      assert_selector "img.voice-stack__avatar[data-user-id='#{users(:david).id}']", wait: BROADCAST_WAIT
    end
    within ".room-header__actions" do
      assert_selector ".voice-stack--live", wait: BROADCAST_WAIT
      assert_selector ".voice-stack__count", text: "1", wait: BROADCAST_WAIT
    end

    grant.revoke!

    within "##{dom_id(direct_room, :list)}" do
      assert_no_selector ".voice-stack--live", wait: BROADCAST_WAIT
      assert_no_selector "img.voice-stack__avatar", wait: BROADCAST_WAIT
    end
    within ".room-header__actions" do
      assert_no_selector ".voice-stack--live", wait: BROADCAST_WAIT
    end
  end

  test "the sidebar aggregate poll clears quietly expired grants" do
    visit room_path(@room)
    wait_for_cable_connection

    sidebar_stack = find("##{dom_id(@room, :list)} .voice-stack", visible: :all)
    assert_equal "0", sidebar_stack["data-huddle-participants-interval-value"]
    header_stack = find(".room-header__actions .voice-stack", visible: :all)
    assert_equal "15000", header_stack["data-huddle-participants-interval-value"]

    observe_turbo_stream_renders
    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: @room.memberships.find_by!(user: users(:david)))
    wait_for_issuance_broadcast
    record_seen_and_deliver(grant)

    within "##{dom_id(@room, :list)}" do
      assert_selector ".voice-stack__count", text: "1", wait: BROADCAST_WAIT
    end

    page.execute_script(<<~JS)
      window.presenceFetches = 0
      window.fetch = ((originalFetch) => (...args) => {
        const url = String(args[0] && args[0].url || args[0])
        if (url.includes("/users/huddle_presence")) window.presenceFetches++
        return originalFetch(...args)
      })(window.fetch.bind(window))
    JS

    # The grant quietly expires: no broadcast fires, so the sidebar row goes
    # stale until the next aggregate poll.
    grant.update_columns(last_seen_at: 1.minute.ago)
    assert_selector "##{dom_id(@room, :list)} .voice-stack__count", text: "1", wait: 0

    within "##{dom_id(@room, :list)}" do
      assert_no_selector ".voice-stack--live", wait: 30
    end
    assert_operator page.evaluate_script("window.presenceFetches"), :>=, 1,
      "the sidebar never ran its aggregate presence poll"
  end

  test "the sidebar aggregate poll runs on connect and skips in-flight refreshes" do
    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: @room.memberships.find_by!(user: users(:david)))
    grant.record_seen!

    visit room_path(@room)
    wait_for_cable_connection

    within "##{dom_id(@room, :list)}" do
      assert_selector ".voice-stack--live .voice-stack__count", text: "1"
    end

    # The grant quietly expires: no broadcast fires, so the sidebar row goes
    # stale until a poll runs.
    grant.update_columns(last_seen_at: 1.minute.ago)
    assert_selector "##{dom_id(@room, :list)} .voice-stack__count", text: "1", wait: 0

    page.execute_script(<<~JS)
      window.presenceFetches = 0
      window.fetch = ((originalFetch) => (...args) => {
        const url = String(args[0] && args[0].url || args[0])
        if (url.includes("/users/huddle_presence")) window.presenceFetches++
        return originalFetch(...args)
      })(window.fetch.bind(window))
    JS

    # A reconnected sidebar (restored frame, fresh subscription) polls
    # immediately through the real connect() instead of waiting out the
    # 15-second interval; the header stack's own interval cannot explain a
    # clear this fast, and it polls a different URL anyway.
    page.execute_script(<<~JS)
      const element = document.querySelector('[data-controller~="huddle-presence"]')
      const controller = window.Stimulus.getControllerForElementAndIdentifier(element, "huddle-presence")
      controller.disconnect()
      controller.connect()
    JS

    within "##{dom_id(@room, :list)}" do
      assert_no_selector ".voice-stack--live", wait: 10
    end
    assert_operator page.evaluate_script("window.presenceFetches"), :>=, 1,
      "reconnecting the sidebar never ran its aggregate presence poll"

    # Two refreshes issued back to back share one request: the second sees
    # the first still in flight and skips itself.
    page.execute_script(<<~JS)
      window.presenceFetches = 0
      const element = document.querySelector('[data-controller~="huddle-presence"]')
      const controller = window.Stimulus.getControllerForElementAndIdentifier(element, "huddle-presence")
      controller.refresh()
      controller.refresh()
    JS
    Timeout.timeout(10) do
      sleep 0.05 until page.evaluate_script("window.presenceFetches") >= 1
    end
    # No settle sleep: the two refresh() calls ran in one synchronous
    # script block, and the in-flight guard runs before refresh's first
    # await, so the second already decided to skip (or wrongly fetched)
    # before the first fetch above could be observed. The count is final;
    # the 15 s interval cannot add another in this test.
    assert_equal 1, page.evaluate_script("window.presenceFetches"),
      "back-to-back refreshes issued duplicate aggregate polls"
  end

  test "a removed sidebar stack clears once but keeps accepting updates while the header latches" do
    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: @room.memberships.find_by!(user: users(:david)))
    grant.record_seen!

    visit room_path(@room)
    wait_for_cable_connection

    within "##{dom_id(@room, :list)}" do
      assert_selector ".voice-stack--live .voice-stack__count", text: "1"
    end
    within ".room-header__actions" do
      assert_selector ".voice-stack--live .voice-stack__count", text: "1"
    end

    # A 404 on one stack dispatches a removal for the shared participants
    # URL; both stacks clear.
    page.execute_script(<<~JS, participants_room_huddle_path(@room))
      const [ url ] = arguments
      window.dispatchEvent(new CustomEvent("huddle-participants:removed", { detail: { url } }))
    JS

    within "##{dom_id(@room, :list)}" do
      assert_no_selector ".voice-stack--live"
    end
    within ".room-header__actions" do
      assert_no_selector ".voice-stack--live"
    end

    # The next aggregate update revives the externally fed sidebar stack, but
    # the polling header stack stays latched.
    david = users(:david)
    page.execute_script(<<~JS, participants_room_huddle_path(@room), [ { id: david.id, name: david.name, avatar_url: fresh_user_avatar_path(david) } ])
      const [ url, participants ] = arguments
      window.dispatchEvent(new CustomEvent("huddle-participants:updated", { detail: { url, participants } }))
    JS

    within "##{dom_id(@room, :list)}" do
      assert_selector ".voice-stack--live .voice-stack__count", text: "1"
      assert_selector "img.voice-stack__avatar[data-user-id='#{david.id}']"
    end
    within ".room-header__actions" do
      assert_no_selector ".voice-stack--live"
    end
  end

  test "the aggregate poller skips while hidden and fetches on becoming visible" do
    visit room_path(@room)
    wait_for_cable_connection
    count_presence_polls
    refresh_presence_poller

    Timeout.timeout(10) do
      sleep 0.05 until presence_poll_count >= 1
    end

    set_visibility_state("hidden")
    hidden_polls = presence_poll_count
    refresh_presence_poller
    sleep 0.3
    assert_equal hidden_polls, presence_poll_count

    set_visibility_state("visible")
    page.execute_script("document.dispatchEvent(new Event('visibilitychange'))")

    Timeout.timeout(10) do
      sleep 0.05 until presence_poll_count > hidden_polls
    end
  ensure
    restore_visibility_state
  end

  private
    # record_seen! refreshes presence through a job; run it inline so the
    # browser receives the stacks without a worker.
    def record_seen_and_deliver(grant)
      perform_enqueued_jobs only: Huddle::BroadcastPresenceJob do
        grant.record_seen!
      end
    end

    # Records every Turbo Stream render as "action:target" so the test can
    # wait for a specific broadcast to land instead of sleeping a fixed time.
    def observe_turbo_stream_renders
      page.execute_script(<<~JS)
        window.presenceObservedStreams = []
        document.addEventListener("turbo:before-stream-render", event => {
          window.presenceObservedStreams.push(`${event.detail.newStream.action}:${event.detail.newStream.target}`)
        })
      JS
    end

    # The test cable adapter delivers over a thread pool, so back-to-back
    # presence broadcasts can arrive out of order and the issuance render
    # (nobody in the huddle yet) would win over the sighting render. Wait for
    # the issuance render to arrive before recording the sighting.
    def wait_for_issuance_broadcast(after: 0, room: @room)
      Timeout.timeout(10) do
        sleep 0.05 until header_presence_renders(room: room) > after
      end
    end

    def header_presence_renders(room: @room)
      expected = "replace:#{dom_id(room, :header_voice_participants)}"
      page.evaluate_script("window.presenceObservedStreams").count(expected)
    end

    def count_presence_polls
      page.execute_script(<<~JS)
        window.presencePollCount = 0
        const nativePollFetch = window.fetch.bind(window)
        window.fetch = (...args) => {
          const input = args[0]
          const url = typeof input === "string" ? input : input.url
          if (new URL(url, window.location.origin).pathname === "/users/huddle_presence") {
            window.presencePollCount += 1
          }
          return nativePollFetch(...args)
        }
      JS
    end

    def presence_poll_count
      page.evaluate_script("window.presencePollCount")
    end

    def refresh_presence_poller
      page.execute_script(<<~JS)
        window.Stimulus
          .getControllerForElementAndIdentifier(
            document.querySelector('[data-controller~="huddle-presence"]'), "huddle-presence")
          .refresh()
      JS
    end

    # Shadows the prototype getter for the test, then deletes the shadow
    # so the real visibility state shows through again.
    def set_visibility_state(state)
      page.execute_script(<<~JS, state)
        Object.defineProperty(document, "visibilityState", {
          configurable: true,
          get: () => arguments[0]
        })
      JS
    end

    def restore_visibility_state
      page.execute_script("delete document.visibilityState")
    rescue StandardError
      nil
    end
end
