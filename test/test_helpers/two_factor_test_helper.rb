module TwoFactorTestHelper
  def enroll_two_factor!(user)
    TwoFactorCredential.create!(user: user,
      secret: TwoFactorCredential.generate_secret, confirmed_at: Time.current)
  end

  def totp_code_for(credential, at: Time.current)
    ROTP::TOTP.new(credential.secret).at(at)
  end

  # Marks the user's newest session two-factor satisfied: the same state
  # the test-only sign-in route produces. For tests whose sign-in happens
  # through another flow under test (Google, transfer) with an unenrolled
  # user, so follow-up requests exercise that flow instead of setup.
  def satisfy_two_factor!(user)
    user.sessions.order(:id).last&.mark_two_factor_verified!
  end

  # Rails rate_limit captures its store (Rails.cache, a null store in the
  # test environment) when the controller class loads, so swapping
  # Rails.cache cannot enable it afterwards. Delegate increments to a
  # memory backend instead: the limiter, keys, and windows under test
  # stay real, only the throwaway backend changes.
  def with_rate_limit_store
    backend = ActiveSupport::Cache::MemoryStore.new
    cache = Rails.cache
    cache.define_singleton_method(:increment) do |name, amount = 1, **options|
      backend.increment(name, amount, **options)
    end
    yield
  ensure
    cache.singleton_class.remove_method(:increment)
  end
end
