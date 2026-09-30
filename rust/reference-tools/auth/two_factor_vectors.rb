require_relative "../support"

class TwoFactorVectors
  include ReferenceTools

  SECRET = "JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP".b

  def run
    travel_to NOW
    reset_database!
    credential = TwoFactorCredential.create!(user: @david, secret: SECRET)
    setup_secret = TwoFactorSetupSecret.issue_for!(@david.sessions.start!(user_agent: "ws9", ip_address: "127.0.0.1"))
    setup_secret.update!(secret: SECRET)
    totp = credential.totp
    now = NOW.to_i
    codes = [-90, -60, -31, -30, -1, 0, 29, 30, 59, 60, 90].map do |offset|
      { at: now + offset, code: totp.at(now + offset) }
    end
    checks = []
    [0, 1, 29, 30, 31].each do |offset|
      codes.each do |vector|
        [nil, now - 30, now, now + 30].each do |after|
          checks << { at: now + offset, code: vector[:code], after: after,
            matched_at: totp.verify(vector[:code], drift_ahead: 30, drift_behind: 30, after: after, at: now + offset) }
        end
      end
    end
    [nil, "", "   ", "000000", "12345", "1234567", "123-456", "１２３４５６", "\u00a0#{totp.at(now)}", "#{totp.at(now)[0, 3]} \t\n#{totp.at(now)[3, 3]}", "#{totp.at(now)}\v\f\r"].each do |code|
      credential.update!(last_totp_at: nil)
      accepted = credential.verify_code(code)
      checks << { at: now, code: code, after: nil, matched_at: accepted ? credential.reload.last_totp_at : nil }
    end
    collision_at = 14207940
    collision_code = totp.at(collision_at)
    raise "collision fixture changed" unless collision_code == totp.at(collision_at - 30)
    # ROTP accepts lowercase and '=' anywhere, discarding incomplete trailing bits.
    secrets = [SECRET, SECRET.downcase, "JB=SWY3DPEHPK3PXP", "A", "", "invalid!", " JBSWY3DP"]
    decoded = secrets.map do |secret|
      begin
        { secret: secret, code: ROTP::TOTP.new(secret).at(now) }
      rescue ROTP::Base32::Base32Error => error
        { secret: secret, error: error.class.name }
      end
    end
    backup_inputs = [nil, "", "  --\t", "ABCD-Ef1234", "a\tb\nc\vd\fe\rf", "\u00a0ABC\u2003", "İÅΣ"]
    backups = backup_inputs.map { |input| { input: input, normalized: TwoFactorBackupCode.normalize(input), digest: TwoFactorBackupCode.digest(input) } }
    cookie, header = write_cookie(env: { "HTTPS" => "on" }) do |jar|
      jar.signed[:two_factor_remember] = { value: "ws9-fixture-remember-token", expires: 30.days.from_now,
        httponly: true, secure: true, same_site: :lax }
    end
    output = {
      reference: "d7c7de92", secret_key_base: ENV.fetch("SECRET_KEY_BASE"), now: now, secret: SECRET,
      codes: codes, checks: checks, base32: decoded, backups: backups,
      collision: { at: collision_at, code: collision_code,
        matched_at: totp.verify(collision_code, drift_ahead: 30, drift_behind: 30, at: collision_at) },
      provisioning_uri: credential.provisioning_uri, formatted_secret: credential.formatted_secret,
      provisioning: ["david@example.com", " User:Name+tag@example.com \t\0", "é@example.com", "a~*b@example.com"].map { |email| { email: email, uri: totp.provisioning_uri(email) } },
      credential_ciphertext: credential.read_attribute_before_type_cast(:secret),
      setup_ciphertext: setup_secret.read_attribute_before_type_cast(:secret),
      remember: { token: "ws9-fixture-remember-token", raw: cookie, header: header,
        expires_at: (NOW + 30.days).to_i }
    }
    File.write(File.join(vectors_dir, "two_factor.json"), JSON.pretty_generate(output) + "\n")
    puts "WS9 vectors: #{codes.length} TOTP codes, #{checks.length} verification cases, #{decoded.length} base32 cases, #{backups.length} backup normalization cases"
  ensure
    travel_back
  end
end

TwoFactorVectors.new.run
