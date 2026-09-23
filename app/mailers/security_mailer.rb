# Security notifications: new-device sign-in alerts. Mail is opt-in
# infrastructure: configured? is true only when SMTP, a sender, and an
# absolute-URL host are all present, and the alert is skipped otherwise
# (the inbox item is still created). See docs/security.md.
class SecurityMailer < ApplicationMailer
  def self.configured?
    ENV["SMTP_ADDRESS"].present? && ENV["MAILER_FROM"].present? &&
      Rails.application.routes.default_url_options[:host].present?
  end

  def self.from_address
    ENV["MAILER_FROM"].presence || "Smartfire <noreply@example.com>"
  end

  # The delivery job runs after the sign-in commits, so the session
  # may already be revoked (signed out, expired, pruned) when this
  # runs: skip without sending rather than failing on a nil session.
  # Not calling mail makes delivery a no-op (NullMail).
  def new_sign_in_alert(activity_item)
    @item = activity_item
    @user = activity_item.user
    @session = activity_item.source
    return if @session.nil?

    mail(
      from: SecurityMailer.from_address,
      to: email_address_with_name(@user.email_address, @user.name),
      subject: "New sign-in to your Smartfire account"
    )
  end
end
