require "application_system_test_case"

class PeopleGroupDmsTest < ApplicationSystemTestCase
  setup do
    sign_in "david@37signals.com"
  end

  test "clicking a message author opens their profile card and Message lands in the DM" do
    visit room_path(rooms(:designers))

    within "#message_#{messages(:first).id}" do
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

    author_button = find("#message_#{messages(:first).id} .message__author button")
    author_button.send_keys(:enter)

    assert_selector "#profile-card-popover:not([hidden])", wait: 10
    assert_selector "#user_card .profile-card__name", text: "Jason"

    find("#profile-card-popover").send_keys(:escape)
    assert_selector "#profile-card-popover[hidden]", visible: :all, wait: 10
    assert_equal "Jason", page.evaluate_script("document.activeElement.textContent.trim()")
  end

  test "multi-selecting three people in the directory lands in their group DM" do
    visit users_path

    check "Select Jason"
    check "Select Kevin"
    check "Select JZ"

    within "[data-multi-select-target='bar']" do
      assert_selector "button", text: "Message (3)"
      click_button "Message (3)"
    end

    room = Rooms::Direct.find_for([ users(:david), users(:jason), users(:kevin), users(:jz) ])
    assert_current_path room_path(room), wait: 10
    assert_selector ".room--current", text: "Jason, JZ, Kevin"
  end

  test "the member panel multi-select starts a huddle with exactly that set" do
    visit room_path(rooms(:designers))
    click_button "Show members"

    assert_selector "#channel-members .member-panel__member", minimum: 3, wait: 10
    check "Select Jason"
    check "Select Kevin"

    within "#channel-members [data-multi-select-target='bar']" do
      assert_selector "button", text: "Start huddle (2)"
      click_button "Start huddle (2)"
    end

    room = Rooms::Direct.find_for([ users(:david), users(:jason), users(:kevin) ])
    assert_current_path room_path(room), wait: 10
    assert_match(/huddle=start/, current_url)
  end

  test "agents are selectable for messages but excluded from huddles" do
    visit users_path

    check "Select Bender Bot"

    within "[data-multi-select-target='bar']" do
      assert_selector "button", text: "Message (1)"
      assert_selector "[data-multi-select-target='huddleButton'][disabled]", text: "Start huddle (0)"
      assert_text "Agents can't join huddles."
    end
  end

  test "group members rename, add, and leave with system messages in the timeline" do
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
    assert_text "David renamed the group to Weekend Plans"
    assert_text "David added JZ to the group"

    visit edit_rooms_direct_path(room)
    accept_confirm { click_button "Leave" }

    assert_current_path root_path, wait: 10
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

  private
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
