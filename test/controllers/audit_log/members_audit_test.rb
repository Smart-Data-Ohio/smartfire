require "test_helper"

class AuditLog::MembersAuditTest < ActionDispatch::IntegrationTest
  include GoogleCalendarTestHelper

  setup do
    sign_in :david
  end

  test "email change is recorded with before and after" do
    assert_difference -> { AuditLog.where(action: "user.email.change").count }, +1 do
      put user_profile_url, params: { user: { email_address: "david@smartdata.net", current_password: "secret123456" } }
    end

    entry = AuditLog.where(action: "user.email.change").last
    assert_equal users(:david).id, entry.actor_id
    assert_equal users(:david).id, entry.target_id
    assert_equal [ "david@37signals.com", "david@smartdata.net" ], entry.details["email_address"]
  end

  test "failed email change is not recorded" do
    assert_no_difference -> { AuditLog.where(action: "user.email.change").count } do
      put user_profile_url, params: { user: { email_address: "nope@smartdata.net", current_password: "wrong" } }
    end

    assert_response :unprocessable_entity
  end

  test "password change is recorded without the password" do
    assert_difference -> { AuditLog.where(action: "user.password.change").count }, +1 do
      put user_profile_url, params: { user: { password: "brand-new-secret" } }
    end

    entry = AuditLog.where(action: "user.password.change").last
    assert_equal users(:david).id, entry.actor_id
    assert_no_match "brand-new-secret", entry.details.to_json
  end

  test "name-only profile edit writes no security rows" do
    assert_no_difference -> { AuditLog.where(action: [ "user.email.change", "user.password.change" ]).count } do
      put user_profile_url, params: { user: { name: "Dave", password: "" } }
    end

    assert_redirected_to user_profile_url
  end

  test "admin role change is recorded with before and after" do
    assert_difference -> { AuditLog.where(action: "user.role.change").count }, +1 do
      put account_user_url(users(:kevin)), params: { user: { role: "administrator" } }
    end

    entry = AuditLog.where(action: "user.role.change").last
    assert_equal users(:david).id, entry.actor_id
    assert_equal users(:kevin).id, entry.target_id
    assert_equal [ "member", "administrator" ], entry.details["role"]
  end

  test "unchanged role writes no row" do
    assert_no_difference -> { AuditLog.where(action: "user.role.change").count } do
      put account_user_url(users(:david)), params: { user: { role: "administrator" } }
    end
  end

  test "ban and unban are recorded" do
    post user_ban_url(users(:kevin))
    assert_redirected_to users(:kevin)

    ban = AuditLog.where(action: "user.ban").last
    assert_equal users(:david).id, ban.actor_id
    assert_equal users(:kevin).id, ban.target_id
    assert_equal [ "active", "banned" ], ban.details["status"]

    delete user_ban_url(users(:kevin))

    unban = AuditLog.where(action: "user.unban").last
    assert_equal users(:david).id, unban.actor_id
    assert_equal [ "banned", "active" ], unban.details["status"]
  end

  test "deactivation is recorded" do
    assert_difference -> { AuditLog.where(action: "user.deactivate").count }, +1 do
      delete account_user_url(users(:kevin))
    end

    entry = AuditLog.where(action: "user.deactivate").last
    assert_equal users(:david).id, entry.actor_id
    assert_equal users(:kevin).id, entry.target_id
    assert_equal [ "active", "deactivated" ], entry.details["status"]
  end

  test "join code reset is recorded without the code" do
    old_code = Current.account.join_code

    assert_difference -> { AuditLog.where(action: "account.join_code.reset").count }, +1 do
      post account_join_code_url
    end

    assert_not_equal old_code, Current.account.reload.join_code
    entry = AuditLog.where(action: "account.join_code.reset").last
    assert_equal users(:david).id, entry.actor_id
    assert_no_match old_code, entry.details.to_json
    assert_no_match Current.account.join_code, entry.details.to_json
  end

  test "Google sign-in link allow and unlink are recorded" do
    post account_user_google_link_url(users(:kevin))

    allow = AuditLog.where(action: "google.sign_in.link_allow").last
    assert_equal users(:david).id, allow.actor_id
    assert_equal users(:kevin).id, allow.target_id

    GoogleIdentity.create!(user: users(:kevin), subject: "google-sub-kevin", email: "kevin@37signals.com")
    delete account_user_google_link_url(users(:kevin))

    unlink = AuditLog.where(action: "google.sign_in.unlink").last
    assert_equal users(:david).id, unlink.actor_id
    assert_equal users(:kevin).id, unlink.target_id
  end

  test "GitHub connect and disconnect are recorded without the token" do
    stub_github_user("octocat")

    assert_difference -> { AuditLog.where(action: "github.account.connect").count }, +1 do
      post github_connection_url, params: { access_token: "github_pat_pasted" }
    end

    connect = AuditLog.where(action: "github.account.connect").last
    assert_equal users(:david).id, connect.actor_id
    assert_equal({ "github_login" => "octocat" }, connect.details)
    assert_no_match "github_pat_pasted", connect.details.to_json

    assert_difference -> { AuditLog.where(action: "github.account.disconnect").count }, +1 do
      delete github_connection_url
    end

    disconnect = AuditLog.where(action: "github.account.disconnect").last
    assert_equal({ "github_login" => "octocat" }, disconnect.details)
  end

  test "Google Calendar connect and disconnect are recorded without tokens" do
    state = connect_state_from_redirect
    stub_google_code_exchange

    assert_difference -> { AuditLog.where(action: "google.account.connect").count }, +1 do
      get google_callback_path, params: { state:, code: "auth-code" }
    end

    connect = AuditLog.where(action: "google.account.connect").last
    assert_equal users(:david).id, connect.actor_id
    assert_no_match "new-access-token", connect.details.to_json
    assert_no_match "new-refresh-token", connect.details.to_json

    assert_difference -> { AuditLog.where(action: "google.account.disconnect").count }, +1 do
      delete google_connection_path
    end

    assert AuditLog.where(action: "google.account.disconnect").last
  end

  private
    def stub_github_user(login)
      stub_request(:get, "https://api.github.com/user")
        .to_return(status: 200, body: { login: login }.to_json)
    end

    def connect_state_from_redirect
      post google_connect_path
      assert_response :redirect
      Rack::Utils.parse_query(URI(response.location).query)["state"]
    end
end
