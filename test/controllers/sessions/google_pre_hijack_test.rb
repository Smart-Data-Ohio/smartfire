require "test_helper"

# A member who types a new hire's Workspace address into their own profile
# must never receive that person's first Google sign-in.
class Sessions::GooglePreHijackTest < ActionDispatch::IntegrationTest
  include GoogleSignInTestHelper
  include GoogleCalendarTestHelper

  test "a self-changed email is not auto-linked and asks for an administrator" do
    sign_in :kevin
    put user_profile_url, params: { user: { email_address: "newhire@smartdata.net", current_password: "secret123456" } }
    assert_redirected_to user_profile_url
    assert users(:kevin).reload.email_self_changed_at.present?
    delete session_url

    state = start_google_sign_in
    assert_no_difference -> { User.count } do
      assert_no_difference -> { GoogleIdentity.count } do
        complete_google_sign_in(state:, email: "newhire@smartdata.net", hd: "smartdata.net", sub: "google-sub-newhire")
      end
    end

    assert_redirected_to new_session_url
    assert_match "An administrator must link this account", flash[:alert]
    assert_nil users(:kevin).reload.google_identity
    assert_nil cookies[:session_token].presence
  end

  test "an administrator allowing the link lets the next Google sign-in link" do
    users(:kevin).update!(email_address: "kevin@smartdata.net", email_self_changed_at: 1.day.ago)

    sign_in :david
    post account_user_google_link_url(users(:kevin))
    assert_redirected_to edit_account_url
    assert_nil users(:kevin).reload.email_self_changed_at
    delete session_url

    state = start_google_sign_in
    complete_google_sign_in(state:, email: "kevin@smartdata.net", hd: "smartdata.net", sub: "google-sub-kevin")

    assert_redirected_to root_url
    assert_equal users(:kevin).id, GoogleIdentity.find_by!(subject: "google-sub-kevin").user_id
  end

  test "an account from before the rule, with its original email, still auto-links" do
    user = User.create!(name: "Original", email_address: "original@smartdata.net", password: "secret123456", google_email_link_allowed: true)

    state = start_google_sign_in
    complete_google_sign_in(state:, email: "original@smartdata.net", hd: "smartdata.net", sub: "google-sub-original")

    assert_redirected_to root_url
    assert_equal user.id, GoogleIdentity.find_by!(subject: "google-sub-original").user_id
  end

  test "a join-code signup never auto-links by email, even untouched" do
    get join_url(accounts(:signal).join_code)
    post join_url(accounts(:signal).join_code), params: { user: { name: "Pre-claimer", email_address: "newhire@smartdata.net", password: "secret123456" } }
    squatter = User.find_by!(email_address: "newhire@smartdata.net")
    assert_not squatter.google_email_link_allowed?
    assert_nil squatter.email_self_changed_at
    delete session_url

    state = start_google_sign_in
    assert_no_difference -> { GoogleIdentity.count } do
      complete_google_sign_in(state:, email: "newhire@smartdata.net", hd: "smartdata.net", sub: "google-sub-newhire")
    end

    assert_redirected_to new_session_url
    assert_match "An administrator must link this account", flash[:alert]
  end

  test "a Google-provisioned account keeps signing in by subject after a self-change" do
    state = start_google_sign_in
    complete_google_sign_in(state:, email: "alice@smartdata.net", hd: "smartdata.net", sub: "google-sub-alice")
    user = User.find_by!(email_address: "alice@smartdata.net")
    satisfy_two_factor!(user)

    # No password to confirm: the change is allowed but recorded.
    put user_profile_url, params: { user: { email_address: "alice.new@smartdata.net" } }
    assert_redirected_to user_profile_url
    assert user.reload.email_self_changed_at.present?
    delete session_url

    state = start_google_sign_in
    complete_google_sign_in(state:, email: "alice@smartdata.net", hd: "smartdata.net", sub: "google-sub-alice")

    assert_redirected_to root_url
    assert_equal user.id, GoogleIdentity.find_by!(subject: "google-sub-alice").user_id
  end
end
