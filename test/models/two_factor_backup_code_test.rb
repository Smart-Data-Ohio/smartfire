require "test_helper"

class TwoFactorBackupCodeTest < ActiveSupport::TestCase
  setup do
    @credential = enroll_two_factor!(users(:david))
  end

  test "regenerate_set! returns ten plaintext codes stored as digests only" do
    codes = TwoFactorBackupCode.regenerate_set!(@credential)

    assert_equal 10, codes.uniq.size
    assert_equal 10, @credential.backup_codes.unused.count
    codes.each do |code|
      assert @credential.backup_codes.unused.exists?(code_digest: TwoFactorBackupCode.digest(code))
    end
    stored = TwoFactorBackupCode.connection.select_values(
      "SELECT code_digest FROM two_factor_backup_codes WHERE two_factor_credential_id = #{@credential.id}")
    codes.each { |code| assert_not_includes stored, code }
  end

  test "regenerate_set! invalidates the old set" do
    old_codes = TwoFactorBackupCode.regenerate_set!(@credential)
    TwoFactorBackupCode.regenerate_set!(@credential)

    old_codes.each do |code|
      assert_not TwoFactorBackupCode.consume!(@credential, code)
    end
  end

  test "consume! spends a code exactly once" do
    codes = TwoFactorBackupCode.regenerate_set!(@credential)

    assert TwoFactorBackupCode.consume!(@credential, codes.first)
    assert_not TwoFactorBackupCode.consume!(@credential, codes.first)
    assert_equal 9, @credential.backup_codes.unused.count
  end

  test "consume! ignores case, spaces, and dashes" do
    codes = TwoFactorBackupCode.regenerate_set!(@credential)
    code = codes.first

    assert TwoFactorBackupCode.consume!(@credential, "#{code[0, 4]}-#{code[4..]}".upcase)
  end

  test "consume! rejects unknown and blank codes" do
    TwoFactorBackupCode.regenerate_set!(@credential)

    assert_not TwoFactorBackupCode.consume!(@credential, "not-a-code")
    assert_not TwoFactorBackupCode.consume!(@credential, "")
    assert_not TwoFactorBackupCode.consume!(@credential, nil)
  end

  test "consume! rejects another credential's code" do
    other = enroll_two_factor!(users(:jason))
    codes = TwoFactorBackupCode.regenerate_set!(other)

    assert_not TwoFactorBackupCode.consume!(@credential, codes.first)
  end
end
