class ApplicationMailer < ActionMailer::Base
  # Confirm this sender with your mail provider when enabling outbound
  # mail (see TwoFactorMailer.mail_configured?).
  default from: "Smartfire <noreply@smartdata.net>"
  layout "mailer"
end
