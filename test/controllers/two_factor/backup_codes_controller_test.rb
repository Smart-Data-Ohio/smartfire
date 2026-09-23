require "test_helper"

class TwoFactor::BackupCodesControllerTest < ActionDispatch::IntegrationTest
  include GoogleSignInTestHelper
  include GoogleCalendarTestHelper

  setup do
    @user = users(:david)
  end

  test "regenerate replaces the set, shows it once, and audits" do
    credential = enroll_two_factor!(@user)
    old_codes = TwoFactorBackupCode.regenerate_set!(credential)
    sign_in @user

    post two_factor_backup_codes_url, params: { reauth: totp_code_for(credential) }

    assert_response :success
    assert_select "#two_factor_backup_codes li", count: 10
    assert_select "a[href='#{user_profile_url}']", text: "Continue"
    assert_equal 10, credential.backup_codes.unused.count
    old_codes.each do |code|
      assert_not TwoFactorBackupCode.consume!(credential, code)
    end
    assert AuditLog.exists?(action: "two_factor.backup_codes.regenerate", target_id: @user.id)
  end

  test "regenerate accepts the password" do
    credential = enroll_two_factor!(@user)
    sign_in @user

    post two_factor_backup_codes_url, params: { reauth: "secret123456" }

    assert_response :success
    assert_equal 10, credential.backup_codes.unused.count
  end

  test "regenerate drops the user's live connections" do
    credential = enroll_two_factor!(@user)
    sign_in @user

    remote_connections = mock
    remote_connections.expects(:disconnect).with(reconnect: true)
    ActionCable.server.stubs(:remote_connections).returns(mock.tap { |m| m.expects(:where).with(current_user: @user).returns(remote_connections) })

    post two_factor_backup_codes_url, params: { reauth: totp_code_for(credential) }

    assert_response :success
  end

  test "regenerate refuses without re-authentication" do
    credential = enroll_two_factor!(@user)
    old_codes = TwoFactorBackupCode.regenerate_set!(credential)
    sign_in @user

    assert_no_difference -> { AuditLog.where(action: "two_factor.backup_codes.regenerate").count } do
      post two_factor_backup_codes_url
    end

    assert_redirected_to user_profile_url
    assert_equal "Enter your authenticator code or password to continue.", flash[:alert]
    old_codes.each do |code|
      assert TwoFactorBackupCode.find_by(code_digest: TwoFactorBackupCode.digest(code)).present?
    end
  end

  test "regenerate refuses a wrong code and a wrong password" do
    credential = enroll_two_factor!(@user)
    old_digests = TwoFactorBackupCode.regenerate_set!(credential).map { |code| TwoFactorBackupCode.digest(code) }
    sign_in @user

    post two_factor_backup_codes_url, params: { reauth: "000000" }
    assert_redirected_to user_profile_url

    post two_factor_backup_codes_url, params: { reauth: "not-the-password" }
    assert_redirected_to user_profile_url

    assert_equal old_digests.sort, credential.backup_codes.unused.pluck(:code_digest).sort
    assert_not AuditLog.exists?(action: "two_factor.backup_codes.regenerate", target_id: @user.id)
  end

  test "regenerate refuses a backup code as re-authentication" do
    credential = enroll_two_factor!(@user)
    codes = TwoFactorBackupCode.regenerate_set!(credential)
    sign_in @user

    post two_factor_backup_codes_url, params: { reauth: codes.first }

    assert_redirected_to user_profile_url
    assert_not AuditLog.exists?(action: "two_factor.backup_codes.regenerate", target_id: @user.id)
  end

  test "regenerate accepts a completed Google re-auth exactly once" do
    credential = enroll_two_factor!(@user)
    identity = link_google_identity!(@user)
    sign_in @user

    state = start_google_reauth
    complete_google_sign_in(state:, sub: identity.subject, email: identity.email)
    assert_redirected_to user_profile_url

    post two_factor_backup_codes_url
    assert_response :success
    assert_equal 10, credential.backup_codes.unused.count

    # Single-use: the next action without a credential is refused again.
    post two_factor_backup_codes_url
    assert_redirected_to user_profile_url
  end

  test "Google-only members re-authenticate with a code, never a password" do
    user = User.create!(name: "Google Only", email_address: "google.only@smartdata.net", password: nil)
    credential = enroll_two_factor!(user)
    link_google_identity!(user)
    sign_in_with_google(user, credential)

    post two_factor_backup_codes_url, params: { reauth: "secret123456" }
    assert_redirected_to user_profile_url
    assert_equal "Enter your authenticator code or confirm with Google to continue.", flash[:alert]

    # The sign-in spent the current TOTP step; re-auth needs the next one.
    travel_to 1.minute.from_now do
      post two_factor_backup_codes_url, params: { reauth: totp_code_for(credential.reload) }
    end
    assert_response :success
  end

  test "regenerate without enrollment redirects to setup" do
    post session_url, params: { email_address: @user.email_address, password: "secret123456" }

    post two_factor_backup_codes_url

    assert_redirected_to two_factor_setup_url
  end

  test "regenerate redirects visitors to sign in" do
    post two_factor_backup_codes_url

    assert_redirected_to new_session_url
  end

  test "create is rate limited per IP" do
    credential = enroll_two_factor!(@user)
    sign_in @user

    with_rate_limit_store do
      10.times { post two_factor_backup_codes_url, params: { reauth: "000000" } }
      assert_redirected_to user_profile_url

      # A valid code is still refused once the limiter trips.
      post two_factor_backup_codes_url, params: { reauth: totp_code_for(credential) }
      assert_redirected_to user_profile_url
      assert_equal "Too many attempts. Try again in a few minutes.", flash[:alert]
    end
  end

  private
    # Signs a Google-only member (no password) all the way in through
    # the real Google + challenge flow.
    def sign_in_with_google(user, credential)
      state = start_google_sign_in
      complete_google_sign_in(state:, sub: user.google_identity.subject, email: user.google_identity.email)
      assert_redirected_to two_factor_challenge_url

      post two_factor_challenge_url, params: { code: totp_code_for(credential) }
      assert_redirected_to root_url
    end
end
