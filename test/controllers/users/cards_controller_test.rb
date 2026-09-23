require "test_helper"

class Users::CardsControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
  end

  test "card shows identity, presence, role, and actions for a peer" do
    jason_session = users(:jason).sessions.create!(user_agent: "test", ip_address: "127.0.0.1")
    WorkspacePresenceLease.establish(user: users(:jason), session: jason_session)

    get user_card_url(users(:jason))

    assert_response :success
    assert_select "turbo-frame#user_card", 1
    assert_select ".profile-card__name", text: "Jason"
    assert_select ".profile-card__presence", text: /Online/
    assert_select ".profile-card__badge", text: "Admin"
    assert_select "form[action='#{rooms_directs_path}']", 2
    assert_select "button", text: "Message"
    assert_select "button", text: "Start call"
    assert_select "a[href='#{user_path(users(:jason))}']", text: "View profile"
    assert_select "button", text: "Copy mention"
  end

  test "offline peers read offline" do
    get user_card_url(users(:kevin))

    assert_response :success
    assert_select ".profile-card__presence", text: /Offline/
  end

  test "your own card offers editing your profile instead" do
    get user_card_url(users(:david))

    assert_response :success
    assert_select ".profile-card__name", text: "David"
    assert_select "a[href='#{user_profile_path}']", text: "Edit profile"
    assert_select "button", text: "Message", count: 0
    assert_select "button", text: "Start call", count: 0
    assert_select "button", text: "Copy mention", count: 0
  end

  test "agents can be messaged but not called" do
    agents(:bender_agent).update!(owner: users(:david))

    get user_card_url(users(:bender))

    assert_response :success
    assert_select ".profile-card__badge", text: "Agent"
    assert_select ".profile-card__owner", text: "Agent owned by David"
    assert_select "button", text: "Message"
    assert_select "button", text: "Start call", count: 0
  end

  test "inactive users show status without message actions" do
    users(:kevin).deactivate

    get user_card_url(users(:kevin))

    assert_response :success
    assert_select ".profile-card__presence", text: /Deactivated/
    assert_select "button", text: "Message", count: 0
    assert_select "a[href='#{user_path(users(:kevin))}']", text: "View profile"
  end

  test "card requires sign-in" do
    delete session_url

    get user_card_url(users(:jason))

    assert_redirected_to new_session_url
  end
end
