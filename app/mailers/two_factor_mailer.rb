# Security mail for two-step sign-in. The app has no outbound mail by
# default; these only send once SMTP is configured, while the matching
# inbox notification always goes.
class TwoFactorMailer < ApplicationMailer
  def self.mail_configured?
    Rails.application.config.action_mailer.smtp_settings.present?
  end

  def lockout_notice(user)
    @user = user
    mail(to: user.email_address, subject: "Several wrong sign-in codes were entered")
  end
end
