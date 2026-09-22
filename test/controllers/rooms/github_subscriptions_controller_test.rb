require "test_helper"

class Rooms::GithubSubscriptionsControllerTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
    @room = rooms(:designers)
    GithubConnectedAccount.create!(user: users(:david), github_login: "david-gh", access_token: "david-token")
    @readable = stub_request(:get, %r{\Ahttps://api\.github\.com/repos/[^/]+/[^/]+\z}).to_return(status: 200, body: "{}")
  end

  test "administrator can subscribe a room with default events" do
    assert_difference -> { @room.github_repository_subscriptions.count }, 1 do
      post room_github_subscriptions_url(@room), params: {
        github_repository_subscription: { full_name: "Rails/Rails" }
      }
    end

    assert_redirected_to edit_rooms_closed_url(@room)

    subscription = @room.github_repository_subscriptions.last
    assert_equal "rails", subscription.owner
    assert_equal "rails", subscription.repo
    assert_equal Github::RepositorySubscription::DEFAULT_EVENTS, subscription.events
    assert_equal users(:david), subscription.created_by
    assert @room.memberships.exists?(user: User.active_bots.find_by!(name: "GitHub"))
  end

  test "administrator can subscribe with an explicit event selection" do
    post room_github_subscriptions_url(@room), params: {
      github_repository_subscription: { full_name: "rails/rails", events: [ "opened", "merged", "" ] }
    }

    assert_redirected_to edit_rooms_closed_url(@room)
    assert_equal %w[ opened merged ], @room.github_repository_subscriptions.last.events
  end

  test "subscribing an open room returns to its edit page" do
    post room_github_subscriptions_url(rooms(:pets)), params: {
      github_repository_subscription: { full_name: "rails/rails" }
    }

    assert_redirected_to edit_rooms_open_url(rooms(:pets))
  end

  test "room creator can subscribe without being an administrator" do
    room = Rooms::Closed.create!(name: "JZ Room", creator: users(:jz))
    room.memberships.grant_to(users(:jz))
    GithubConnectedAccount.create!(user: users(:jz), github_login: "jz-gh", access_token: "jz-token")
    sign_in :jz

    post room_github_subscriptions_url(room), params: {
      github_repository_subscription: { full_name: "rails/rails" }
    }

    assert_redirected_to edit_rooms_closed_url(room)
    assert room.github_repository_subscriptions.exists?(owner: "rails", repo: "rails")
  end

  test "duplicate and malformed subscriptions redirect with an alert" do
    @room.github_repository_subscriptions.create!(owner: "rails", repo: "rails", created_by: users(:david))

    assert_no_difference -> { Github::RepositorySubscription.count } do
      post room_github_subscriptions_url(@room), params: {
        github_repository_subscription: { full_name: "rails/rails" }
      }
    end
    assert_redirected_to edit_rooms_closed_url(@room)
    assert_equal "Could not subscribe: Owner has already been taken.", flash[:alert]

    assert_no_difference -> { Github::RepositorySubscription.count } do
      post room_github_subscriptions_url(@room), params: {
        github_repository_subscription: { full_name: "not-a-repo" }
      }
    end
    assert_redirected_to edit_rooms_closed_url(@room)
    assert flash[:alert].start_with?("Could not subscribe:")
  end

  test "administrator can change events and remove a subscription" do
    subscription = @room.github_repository_subscriptions.create!(
      owner: "rails", repo: "rails", created_by: users(:david))
    bot = User.active_bots.find_by!(name: "GitHub")

    patch room_github_subscription_url(@room, subscription), params: {
      github_repository_subscription: { events: [ "merged", "" ] }
    }

    assert_redirected_to edit_rooms_closed_url(@room)
    assert_equal %w[ merged ], subscription.reload.events

    delete room_github_subscription_url(@room, subscription)

    assert_redirected_to edit_rooms_closed_url(@room)
    assert_not Github::RepositorySubscription.exists?(subscription.id)
    assert_not @room.memberships.exists?(user: bot)
  end

  test "removing one of several subscriptions keeps the bot in the room" do
    first = @room.github_repository_subscriptions.create!(owner: "rails", repo: "rails", created_by: users(:david))
    @room.github_repository_subscriptions.create!(owner: "rails", repo: "propshaft", created_by: users(:david))
    bot = User.active_bots.find_by!(name: "GitHub")

    delete room_github_subscription_url(@room, first)

    assert_redirected_to edit_rooms_closed_url(@room)
    assert @room.memberships.exists?(user: bot)
  end

  test "plain members get forbidden" do
    sign_in :jz
    subscription = @room.github_repository_subscriptions.create!(
      owner: "rails", repo: "rails", created_by: users(:david))

    post room_github_subscriptions_url(@room), params: {
      github_repository_subscription: { full_name: "rails/propshaft" }
    }
    assert_response :forbidden

    patch room_github_subscription_url(@room, subscription), params: {
      github_repository_subscription: { events: [ "merged" ] }
    }
    assert_response :forbidden

    delete room_github_subscription_url(@room, subscription)
    assert_response :forbidden

    assert Github::RepositorySubscription.exists?(subscription.id)
  end

  test "non-members get not found" do
    sign_in :kevin # not a member of the watercooler

    # RoomScoped raises RecordNotFound, which renders 404 outside tests.
    assert_raises(ActiveRecord::RecordNotFound) do
      post room_github_subscriptions_url(rooms(:watercooler)), params: {
        github_repository_subscription: { full_name: "rails/rails" }
      }
    end
  end

  test "direct rooms get not found" do
    post room_github_subscriptions_url(rooms(:david_and_jason)), params: {
      github_repository_subscription: { full_name: "rails/rails" }
    }
    assert_response :not_found
  end

  test "github section renders for administrators but not plain members" do
    @room.github_repository_subscriptions.create!(owner: "rails", repo: "rails", created_by: users(:david))

    get edit_rooms_closed_url(@room)
    assert_response :success
    assert_select "#github-subscriptions", text: /rails\/rails/

    get edit_rooms_open_url(rooms(:pets))
    assert_response :success
    assert_select "#github-subscriptions"

    sign_in :jz
    get edit_rooms_closed_url(@room)
    assert_response :success
    assert_select "#github-subscriptions", count: 0
  end

  test "subscribing checks access with the subscriber's own token and records a verified reader" do
    post room_github_subscriptions_url(@room), params: { github_repository_subscription: { full_name: "Rails/Rails" } }

    assert_requested :get, "https://api.github.com/repos/rails/rails", headers: { "Authorization" => "Bearer david-token" }
    assert @room.github_repository_subscriptions.find_by!(owner: "rails", repo: "rails").reader_verified?
  end

  test "a subscriber whose token cannot read the repository is refused" do
    remove_request_stub(@readable)
    stub_request(:get, "https://api.github.com/repos/acme/secret").to_return(status: 404, body: "{}")
    room = Rooms::Closed.create!(name: "JZ Room", creator: users(:jz))
    room.memberships.grant_to(users(:jz))
    GithubConnectedAccount.create!(user: users(:jz), github_login: "jz-gh", access_token: "jz-token")
    sign_in :jz

    assert_no_difference -> { Github::RepositorySubscription.count } do
      post room_github_subscriptions_url(room), params: { github_repository_subscription: { full_name: "acme/secret", skip_access_check: "1" } }
    end

    assert_redirected_to edit_rooms_closed_url(room)
    assert_equal "Could not subscribe: your linked GitHub account could not confirm it can read acme/secret.", flash[:alert]
  end

  test "a subscriber without a linked GitHub account is refused" do
    users(:david).github_connected_account.destroy!

    assert_no_difference -> { Github::RepositorySubscription.count } do
      post room_github_subscriptions_url(@room), params: { github_repository_subscription: { full_name: "rails/rails" } }
    end

    assert_not_requested @readable
    assert_match "link your GitHub account on your profile", flash[:alert]
    assert_match "Administrators may subscribe without verifying access", flash[:alert]
  end

  test "a disconnected GitHub account cannot vouch for a subscription" do
    users(:david).github_connected_account.mark_disconnected!("GitHub rejected the linked token (401)")

    assert_no_difference -> { Github::RepositorySubscription.count } do
      post room_github_subscriptions_url(@room), params: { github_repository_subscription: { full_name: "rails/rails" } }
    end

    assert_not_requested @readable
  end

  test "administrators may override the check, leaving the subscription unverified" do
    remove_request_stub(@readable)
    stub_request(:get, "https://api.github.com/repos/acme/secret").to_return(status: 404, body: "{}")

    assert_difference -> { Github::RepositorySubscription.count }, 1 do
      post room_github_subscriptions_url(@room), params: { github_repository_subscription: { full_name: "acme/secret", skip_access_check: "1" } }
    end

    assert_redirected_to edit_rooms_closed_url(@room)
    assert_not @room.github_repository_subscriptions.find_by!(owner: "acme", repo: "secret").reader_verified?
  end

  test "the override checkbox renders for administrators only" do
    get edit_rooms_closed_url(@room)
    assert_select "input[name=?]", "github_repository_subscription[skip_access_check]"

    room = Rooms::Closed.create!(name: "JZ Room", creator: users(:jz))
    room.memberships.grant_to(users(:jz))
    sign_in :jz
    get edit_rooms_closed_url(room)
    assert_select "#github-subscriptions"
    assert_select "input[name=?]", "github_repository_subscription[skip_access_check]", count: 0
  end
end
