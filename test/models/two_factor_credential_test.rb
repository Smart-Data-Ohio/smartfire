require "test_helper"

class TwoFactorCredentialTest < ActiveSupport::TestCase
  setup do
    @user = users(:david)
    @credential = TwoFactorCredential.create!(user: @user, secret: TwoFactorCredential.generate_secret)
  end

  test "confirm! enables with a valid code and stamps the step" do
    assert_not @credential.enabled?

    assert @credential.confirm!(totp_code_for(@credential))

    assert @credential.reload.enabled?
    assert @credential.last_totp_at.present?
  end

  test "confirm! rejects a wrong code" do
    assert_not @credential.confirm!("000000")
    assert_not @credential.reload.enabled?
  end

  test "confirm! tolerates spaces in the code" do
    code = totp_code_for(@credential)
    assert @credential.confirm!("#{code[0, 3]} #{code[3, 3]}")
  end

  test "verify_code accepts the adjacent steps (±1 drift)" do
    travel_to Time.current do
      assert @credential.verify_code(totp_code_for(@credential, at: 30.seconds.ago))
    end

    other = TwoFactorCredential.create!(user: users(:jason), secret: TwoFactorCredential.generate_secret)
    travel_to Time.current do
      assert other.verify_code(totp_code_for(other, at: 30.seconds.from_now))
    end
  end

  test "verify_code rejects steps beyond the drift window" do
    travel_to Time.current do
      assert_not @credential.verify_code(totp_code_for(@credential, at: 90.seconds.ago))
      assert_not @credential.verify_code(totp_code_for(@credential, at: 90.seconds.from_now))
    end
  end

  test "verify_code rejects reuse of the last accepted step" do
    code = nil
    travel_to Time.utc(2026, 9, 23, 12, 0, 10) do
      code = totp_code_for(@credential)
      assert @credential.verify_code(code)
      assert_not @credential.verify_code(code)
    end

    # The same step stays spent inside the drift window.
    travel_to Time.utc(2026, 9, 23, 12, 0, 45) do
      assert_not @credential.verify_code(code)
    end
  end

  test "verify_code rejects blank codes" do
    assert_not @credential.verify_code(nil)
    assert_not @credential.verify_code("")
    assert_not @credential.verify_code("   ")
  end

  test "secret is encrypted at rest" do
    stored = TwoFactorCredential.connection.select_value(
      "SELECT secret FROM two_factor_credentials WHERE id = #{@credential.id}")

    assert_not_equal @credential.secret, stored
    assert_equal @credential.secret, @credential.reload.secret
  end

  test "one credential per user" do
    assert_raises ActiveRecord::RecordInvalid do
      TwoFactorCredential.create!(user: @user, secret: TwoFactorCredential.generate_secret)
    end
  end

  test "provisioning uri names the user and issuer" do
    uri = @credential.provisioning_uri

    assert_includes uri, "Smartfire"
    assert_includes uri, URI.encode_uri_component(@user.email_address)
  end

  test "user requires two-factor only when an active human" do
    assert @user.requires_two_factor?
    assert_not users(:bender).requires_two_factor?
  end
end
