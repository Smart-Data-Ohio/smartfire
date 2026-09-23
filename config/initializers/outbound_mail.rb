# Outbound mail (security alerts) sends through SMTP only when SMTP_ADDRESS
# is set; without it the app sends nothing and SecurityMailer.configured?
# stays false. MAILER_FROM is the sender, APP_URL supplies the host for
# absolute links (see default_url_options.rb). Optional: SMTP_PORT (25),
# SMTP_DOMAIN, SMTP_USER_NAME, SMTP_PASSWORD, SMTP_AUTHENTICATION,
# SMTP_ENABLE_STARTTLS ("false" disables; on by default).
if ENV["SMTP_ADDRESS"].present?
  Rails.application.configure do
    config.action_mailer.delivery_method = :smtp
    config.action_mailer.smtp_settings = {
      address: ENV["SMTP_ADDRESS"],
      port: ENV.fetch("SMTP_PORT", "25").to_i,
      domain: ENV["SMTP_DOMAIN"].presence,
      user_name: ENV["SMTP_USER_NAME"].presence,
      password: ENV["SMTP_PASSWORD"].presence,
      authentication: ENV["SMTP_AUTHENTICATION"].presence,
      enable_starttls: ENV.fetch("SMTP_ENABLE_STARTTLS", "true") != "false"
    }.compact
  end
end
