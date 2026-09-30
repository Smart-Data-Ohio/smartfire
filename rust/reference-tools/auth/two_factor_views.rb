# Actual pinned Rails templates/helpers, using WS6's deterministic form-token convention.
require "json"
class GoldenTwoFactorController < TwoFactor::SetupsController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
renderer = GoldenTwoFactorController.renderer.new(http_host: "campfire.test", https: false, "rack.session" => {})
Current.reset
user = User.find(127326141)
credential = TwoFactorCredential.new(user: user, secret: "JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP")
uri = credential.provisioning_uri
codes = %w[abcde12345 fghij67890 klmno12345 pqrst67890 uvwxy12345 zabcd67890 efghi12345 jklmn67890 opqrs12345 tuvwx67890]
goldens = {
  setup: renderer.render(template: "two_factor/setups/show", layout: false, assigns: { credential:, provisioning_uri: uri }),
  challenge: renderer.render(template: "two_factor/challenges/show", layout: false),
  backups: renderer.render(template: "two_factor/backup_codes/show", layout: false, assigns: { backup_codes: codes, continue_url: "http://campfire.test/users/me/profile", signed_out_other_devices: 0 }),
  backups_signed_out: renderer.render(template: "two_factor/backup_codes/show", layout: false, assigns: { backup_codes: codes, continue_url: "http://campfire.test/rooms/486777696", signed_out_other_devices: 2 }),
  uri: uri,
  qr: ApplicationController.helpers.two_factor_qr_code(uri).to_s,
  codes: codes,
  key: credential.formatted_secret
}
File.write(ENV.fetch("WS9_TWO_FACTOR_GOLDENS"), JSON.pretty_generate(goldens) + "\n")
puts "WS9 two-factor views: 4 Rails page bodies and inline QR rendered from the parity seed"
