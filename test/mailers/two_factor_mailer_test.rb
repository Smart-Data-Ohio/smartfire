require "test_helper"

class TwoFactorMailerTest < ActionMailer::TestCase
  test "lockout_notice tells the member what happened" do
    mail = TwoFactorMailer.lockout_notice(users(:david))

    assert_equal [ "david@37signals.com" ], mail.to
    assert_equal "Several wrong sign-in codes were entered", mail.subject
    assert_match "Several wrong sign-in codes were entered for your account.", mail.text_part.body.to_s
    assert_match "Several wrong sign-in codes were entered for your account.", mail.html_part.body.to_s
  end

  test "mail is not configured by default" do
    assert_not TwoFactorMailer.mail_configured?
  end
end
