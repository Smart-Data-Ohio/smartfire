module TwoFactorTestHelper
  def enroll_two_factor!(user)
    TwoFactorCredential.create!(user: user,
      secret: TwoFactorCredential.generate_secret, confirmed_at: Time.current)
  end

  def totp_code_for(credential, at: Time.current)
    totp_code_for_secret(credential.secret, at: at)
  end

  def totp_code_for_secret(secret, at: Time.current)
    ROTP::TOTP.new(secret).at(at)
  end

  # Starts a Google re-auth step-up and returns the state Google would
  # echo back. Pair with GoogleSignInTestHelper#complete_google_sign_in
  # (the callback completion is purpose-agnostic); the member needs a
  # GoogleIdentity whose subject the id_token stubs must match.
  def start_google_reauth
    post two_factor_reauthentication_path
    assert_response :redirect
    query = Rack::Utils.parse_query(URI(response.location).query)
    @last_sign_in_nonce = query["nonce"]
    query["state"]
  end

  def link_google_identity!(user, subject: "google-sub-#{user.id}", email: "#{user.id}@smartdata.net")
    GoogleIdentity.create!(user: user, subject: subject, email: email)
  end

  # Marks the user's newest session two-factor satisfied: the same state
  # the test-only sign-in route produces. For tests whose sign-in happens
  # through another flow under test (Google, transfer) with an unenrolled
  # user, so follow-up requests exercise that flow instead of setup.
  def satisfy_two_factor!(user)
    user.sessions.order(:id).last&.mark_two_factor_verified!
  end

  # Rate limits count against the suite-wide memory store (see
  # test.rb), cleared between tests. Clear it again here so the
  # block's limit assertions start from a fresh window even when the
  # test already made requests.
  def with_rate_limit_store
    ActionController::Base.cache_store.clear
    yield
  end
end
