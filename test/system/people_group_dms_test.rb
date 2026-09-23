require "application_system_test_case"

class PeopleGroupDmsTest < ApplicationSystemTestCase
  setup do
    sign_in "david@37signals.com"
  end

  test "clicking a message author opens their profile card and Message lands in the DM" do
    visit room_path(rooms(:designers))
    wait_for_controller "profile-card"

    within "#message_#{messages(:first).client_message_id}" do
      find(".message__avatar a").click
    end

    assert_selector "#profile-card-popover:not([hidden])", wait: 10
    within "#user_card" do
      assert_selector ".profile-card__name", text: "Jason"
      assert_selector "button", text: "Start call"
      click_button "Message"
    end

    assert_current_path room_path(rooms(:david_and_jason)), wait: 10
    assert_selector ".room--current", text: "Jason"
  end

  test "the profile card opens by keyboard, traps focus, and returns it on Esc" do
    visit room_path(rooms(:designers))
    wait_for_controller "profile-card"

    author_button = find("#message_#{messages(:first).client_message_id} .message__author button")
    author_button.send_keys(:enter)

    assert_selector "#profile-card-popover:not([hidden])", wait: 10
    assert_selector "#user_card .profile-card__name", text: "Jason"

    find(".profile-card-popover__panel").send_keys(:escape)
    assert_selector "#profile-card-popover[hidden]", visible: :all, wait: 10
    assert_equal "BUTTON", page.evaluate_script("document.activeElement.tagName")
    assert_includes page.evaluate_script("document.activeElement.textContent"), "Jason"
  end

  test "Esc with a closed profile card stays unhandled for later listeners" do
    visit room_path(rooms(:designers))
    wait_for_controller "profile-card"

    assert_selector "#profile-card-popover[hidden]", visible: :all
    prevented = page.evaluate_script(<<~JS)
      (() => {
        // Only the card's own handler is under test: the global Esc
        // shortcut marks the room read and yields to theater itself.
        const event = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true })
        const card = window.Stimulus.getControllerForElementAndIdentifier(document.body, "profile-card")
        card.close(event)
        return event.defaultPrevented
      })()
    JS

    assert_not prevented, "a closed profile card must not preventDefault Escape (huddle theater owns it)"
  end

  test "the sidebar avatar trigger opens the profile card by keyboard" do
    visit room_path(rooms(:designers))
    wait_for_controller "profile-card"

    find("#direct_rooms button.profile-card-avatar", match: :first).send_keys(:enter)

    assert_selector "#profile-card-popover:not([hidden])", wait: 10
    assert_selector "#user_card .profile-card__name"
  end

  test "multi-selecting three people in the directory lands in their group DM" do
    visit users_path
    wait_for_controller "multi-select"

    check "select_user_#{users(:jason).id}"
    check "select_user_#{users(:kevin).id}"
    check "select_user_#{users(:jz).id}"

    within "[data-multi-select-target='bar']" do
      assert_selector "button", text: "Message (3)"
      click_button "Message (3)"
    end

    assert_selector ".room--current", text: "Jason, JZ, Kevin", wait: 10
    room = Rooms::Direct.find_for([ users(:david), users(:jason), users(:kevin), users(:jz) ])
    assert_current_path room_path(room)
  end

  test "the member panel multi-select starts a huddle with exactly that set" do
    visit room_path(rooms(:designers))
    wait_for_controller "multi-select"

    # Wide screens open the panel on load (flipping the toggle to Hide).
    click_button "Show members" if page.has_button?("Show members", wait: 5)

    assert_selector "#channel-members .member-panel__member", minimum: 3, wait: 10
    check "select-member-#{users(:jason).id}"
    check "select-member-#{users(:kevin).id}"

    within "#channel-members [data-multi-select-target='bar']" do
      assert_selector "button", text: "Start huddle (2)"
      click_button "Start huddle (2)"
    end

    assert_selector ".room--current", text: "Jason, Kevin", wait: 10
    room = Rooms::Direct.find_for([ users(:david), users(:jason), users(:kevin) ])
    assert_current_path room_path(room, huddle: "start")
  end

  test "shift-click extends the checkbox range" do
    visit users_path
    wait_for_controller "multi-select"

    check "select_user_#{users(:bender).id}"
    box = find("#select_user_#{users(:kevin).id}")
    page.driver.browser.action.key_down(:shift).click(box.native).key_up(:shift).perform

    within "[data-multi-select-target='bar']" do
      assert_selector "button", text: "Message (4)"
      assert_selector "button", text: "Start huddle (3)"
    end
  end

  test "long-press selects a row on touch" do
    visit users_path
    wait_for_controller "multi-select"
    row = find(".people-directory__row", text: "Jason")

    page.execute_script(<<~JS, row)
      const row = arguments[0]
      const touch = new Touch({ identifier: 1, target: row, clientX: 10, clientY: 10 })
      row.dispatchEvent(new TouchEvent("touchstart", { touches: [ touch ], bubbles: true, cancelable: true }))
    JS
    sleep 0.7
    page.execute_script(<<~JS, row)
      arguments[0].dispatchEvent(new TouchEvent("touchend", { bubbles: true, cancelable: true }))
    JS

    assert_checked_field "select_user_#{users(:jason).id}", visible: :all
    within "[data-multi-select-target='bar']" do
      assert_selector "button", text: "Message (1)"
    end
  end

  test "agents are selectable for messages but excluded from huddles" do
    visit users_path
    wait_for_controller "multi-select"

    check "select_user_#{users(:bender).id}"

    within "[data-multi-select-target='bar']" do
      assert_selector "button", text: "Message (1)"
      assert_selector "[data-multi-select-target='huddleButton'][disabled]", text: "Start huddle (0)"
      assert_text "Agents can't join huddles."
    end
  end

  test "start huddle keeps agents in the DM but out of the call" do
    visit users_path
    wait_for_controller "multi-select"

    check "select_user_#{users(:jason).id}"
    check "select_user_#{users(:bender).id}"

    within "[data-multi-select-target='bar']" do
      assert_selector "button", text: "Message (2)"
      assert_selector "button", text: "Start huddle (1)"
      assert_text "1 agent stays in the DM but won't be rung."
    end
  end

  test "the new-DM picker types, picks suggestions, and messages the picked set" do
    join_room rooms(:designers)
    click_link "New direct message"

    within "#direct_rooms_control" do
      find("[data-autocomplete-target='input']").fill_in(with: "Kev")
    end
    assert_selector "suggestion-option", text: "Kevin"
    find("suggestion-option", text: "Kevin").click

    within "#direct_rooms_control" do
      assert_selector ".autocomplete__pill", text: "Kevin"
      find("[data-autocomplete-target='input']").fill_in(with: "Jas")
    end
    assert_selector "suggestion-option", text: "Jason"
    find("suggestion-option", text: "Jason").click

    within "#direct_rooms_control" do
      assert_selector ".autocomplete__pill", text: "Jason"
      find("[data-autocomplete-target='input']").ancestor("form").find("button[type='submit']").click
    end

    assert_selector ".room--current", text: "Jason, Kevin", wait: 10
    room = Rooms::Direct.find_for([ users(:david), users(:jason), users(:kevin) ])
    assert_current_path room_path(room)
  end

  test "group members rename, add, and leave with system notes in the timeline" do
    room = Current.set(user: users(:david)) do
      Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin) ])
    end
    visit room_path(room)

    find("nav a[href='#{edit_rooms_direct_path(room)}']").click
    fill_in "Group name", with: "Weekend Plans"
    click_button "Save"
    assert_field "Group name", with: "Weekend Plans"

    check "add_member_#{users(:jz).id}"
    click_button "Add to group"
    assert_selector ".directs--edit", text: "JZ"

    visit room_path(room)
    assert_selector ".room--current", text: "Weekend Plans"
    assert_selector ".message__system-note", text: "renamed the group to Weekend Plans"
    assert_selector ".message__system-note", text: "added JZ to the group"
    assert_text(/David\s+renamed the group to Weekend Plans/)
    assert_text(/David\s+added JZ to the group/)

    visit edit_rooms_direct_path(room)
    accept_confirm { click_button "Leave" }

    # Root forwards to the last accessible room; the point is the leaver
    # is out of the group.
    assert_no_current_path room_path(room), wait: 10
    assert_not room.reload.user_ids.include?(users(:david).id)
  end

  test "starting a group huddle rings every other member" do
    with_huddle_environment do
      using_session :recipient do
        sign_in "jason@37signals.com"
        visit room_path(rooms(:designers))
        wait_for_cable_connection

        room = Current.set(user: users(:david)) do
          Rooms::Direct.find_or_create_for([ users(:david), users(:jason), users(:kevin) ])
        end
        HuddleGrant.issue!(session: sessions(:david_safari), membership: room.memberships.find_by!(user: users(:david)))

        assert_selector "#huddle-invitation:not([hidden])", text: "David started a huddle", wait: 10
      end

      assert ActivityItem.exists?(user: users(:jason), event_type: "huddle_started")
      assert ActivityItem.exists?(user: users(:kevin), event_type: "huddle_started")
    end
  end

  test "Esc closes only the profile card inside the mobile member panel" do
    page.current_window.resize_to(390, 844)
    visit room_path(rooms(:designers))
    wait_for_controller "member-panel"
    wait_for_controller "profile-card"

    click_button "Show members" if page.has_button?("Show members", wait: 5)
    assert_selector "#channel-members .member-panel__member", minimum: 3, wait: 10

    trigger = "#channel-members [data-member-id='#{users(:kevin).id}'] button.profile-card-name"
    find(trigger).click
    assert_selector "#profile-card-popover:not([hidden])", wait: 10

    find(".profile-card-popover__panel").send_keys(:escape)

    assert_selector "#profile-card-popover[hidden]", visible: :all, wait: 10
    assert_selector "#channel-members", visible: true
    assert_button "Close members"
    assert_focused trigger
  ensure
    page.current_window.resize_to(1400, 1400)
  end

  test "Tab cycles within the profile card opened from the member panel" do
    page.current_window.resize_to(390, 844)
    visit room_path(rooms(:designers))
    wait_for_controller "member-panel"
    wait_for_controller "profile-card"

    click_button "Show members" if page.has_button?("Show members", wait: 5)
    assert_selector "#channel-members .member-panel__member", minimum: 3, wait: 10

    find("#channel-members [data-member-id='#{users(:kevin).id}'] button.profile-card-name").click
    assert_selector "#profile-card-popover:not([hidden])", wait: 10
    assert_selector "#user_card .profile-card__name", text: "Kevin"

    # Opening the card focuses its panel; Tab must then walk the card's
    # own controls instead of being yanked back into the member panel.
    find(".profile-card-popover__panel").send_keys(:tab)
    assert_focused ".profile-card-popover__close"
    find(".profile-card-popover__close").send_keys(:tab)
    assert_focused "#user_card .profile-card__actions form:nth-of-type(1) button"
    find("#user_card .profile-card__actions form:nth-of-type(1) button").send_keys(:tab)
    assert_focused "#user_card .profile-card__actions form:nth-of-type(2) button"
  ensure
    page.current_window.resize_to(1400, 1400)
  end

  private
    # Controllers lazy-load when their element appears; under load the
    # module can lag behind the first interaction, so wait for it.
    def wait_for_controller(identifier)
      sleep 0.05 until page.evaluate_script(<<~JS)
        (() => {
          const element = document.querySelector("[data-controller~='#{identifier}']")
          return !!(element && window.Stimulus && window.Stimulus.getControllerForElementAndIdentifier(element, "#{identifier}"))
        })()
      JS
    end

    def with_huddle_environment(&block)
      names = Huddle::REQUIRED_ENVIRONMENT
      original = ENV.values_at(*names)
      ENV["LIVEKIT_URL"] = "wss://huddle.example.test"
      ENV["LIVEKIT_INTERNAL_URL"] = "ws://livekit.example.test:7880"
      ENV["LIVEKIT_API_KEY"] = "test-api-key"
      ENV["LIVEKIT_API_SECRET"] = "test-api-secret"
      ENV["LIVEKIT_GATEWAY_SECRET"] = "test-gateway-secret"

      block.call
    ensure
      names.zip(original).each { |name, value| ENV[name] = value }
    end
end
