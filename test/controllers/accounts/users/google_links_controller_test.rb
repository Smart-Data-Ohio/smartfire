require "test_helper"

class Accounts::Users::GoogleLinksControllerTest < ActionDispatch::IntegrationTest
  setup do
    users(:kevin).update!(email_address: "kevin@smartdata.net", email_self_changed_at: 1.day.ago)
    GoogleIdentity.create!(user: users(:jz), subject: "google-sub-jz", email: "jz@smartdata.net", domain: "smartdata.net")
  end

  test "administrators allow a Google link for a self-changed email" do
    sign_in :david

    post account_user_google_link_url(users(:kevin))

    assert_redirected_to edit_account_url
    assert_nil users(:kevin).reload.email_self_changed_at
  end

  test "administrators unlink a Google identity" do
    sign_in :david

    assert_difference -> { GoogleIdentity.count }, -1 do
      delete account_user_google_link_url(users(:jz))
    end

    assert_redirected_to edit_account_url
    assert_nil users(:jz).reload.google_identity
  end

  test "members cannot allow or remove Google links" do
    sign_in :jz

    post account_user_google_link_url(users(:kevin))
    assert_response :forbidden
    assert users(:kevin).reload.email_self_changed_at.present?

    assert_no_difference -> { GoogleIdentity.count } do
      delete account_user_google_link_url(users(:jz))
    end
    assert_response :forbidden
  end

  test "bots have no Google link to manage" do
    sign_in :david

    assert_raises(ActiveRecord::RecordNotFound) { post account_user_google_link_url(users(:bender)) }
  end

  test "the account page offers the controls to administrators only" do
    sign_in :david
    get edit_account_url
    assert_select "form[action=?]", account_user_google_link_path(users(:kevin))
    assert_select "form[action=?]", account_user_google_link_path(users(:jz))

    sign_in :jz
    get edit_account_url
    assert_select "form[action=?]", account_user_google_link_path(users(:kevin)), count: 0
    assert_select "form[action=?]", account_user_google_link_path(users(:jz)), count: 0
  end
end
