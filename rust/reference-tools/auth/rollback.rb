require_relative "../support"

class TwoFactorRollback
  include ReferenceTools

  def check(condition, message)
    raise message unless condition
  end

  def run
    directory = ENV.fetch("WS9_ROLLBACK_DIR")
    data = JSON.parse(File.read(File.join(directory, "manifest.json")))
    travel_to Time.at(data.fetch("now")).utc
    credential = TwoFactorCredential.find(data.fetch("credential_id"))
    setup = TwoFactorSetupSecret.find(data.fetch("setup_id"))
    pending = TwoFactorCredential.find(data.fetch("pending_credential_id"))
    device = TwoFactorRememberedDevice.find(data.fetch("remembered_id"))
    verified = Session.find_by!(token: data.fetch("verified_session_token"))
    unverified = Session.find_by!(token: data.fetch("unverified_session_token"))
    user_device = credential.user.user_devices.find_by!(device_id: "ws9-device")
    rows = [credential, setup, pending, device, verified, unverified, user_device] + credential.backup_codes.to_a
    rows.each { |row| check(row.valid?, "#{row.class}: #{row.errors.full_messages.inspect}") }
    check(credential.secret == data.fetch("enrollment_secret"), "Rust credential decrypt")
    check(credential.enabled? && !pending.enabled?, "enrollment state")
    check(setup.secret == data.fetch("pending_secret"), "Rust setup decrypt")
    check(verified.two_factor_verified_at.to_i == data.fetch("now"), "session verification timestamp")
    check(!unverified.two_factor_verified?, "unverified session remains unverified")
    check(verified.device_id == "ws9-device", "session device")
    check(!credential.verify_code(data.fetch("enrollment_code")), "enrollment replay")
    check(read_cookie(:signed, :two_factor_remember, data.fetch("remember_cookie")) == data.fetch("remembered_token"), "Rust remember cookie")
    check(read_cookie(:signed, :device_id, data.fetch("device_cookie")) == "ws9-device", "Rust device cookie")
    check(TwoFactorRememberedDevice.find_valid(data.fetch("remembered_token"), credential.user).id == device.id, "Rust device digest")
    codes = data.fetch("backup_codes")
    check(!TwoFactorBackupCode.consume!(credential, codes[0]), "Rust-spent backup reuse")
    check(TwoFactorBackupCode.consume!(credential, codes[1].upcase), "Rust backup digest")
    check(!TwoFactorBackupCode.consume!(credential, codes[1]), "Rails-spent backup reuse")
    travel_back
    travel_to Time.at(data.fetch("now") + 30).utc
    check(credential.verify_code(credential.totp.now), "Rails verifies Rust TOTP")
    setup.update!(secret: TwoFactorCredential.generate_secret)
    cookie, _header = write_cookie(env: { "HTTPS" => "on" }) do |jar|
      jar.signed[:two_factor_remember] = { value: data.fetch("remembered_token"), expires: 30.days.from_now,
        httponly: true, secure: true, same_site: :lax }
    end
    File.write(File.join(directory, "rails-readback.json"), JSON.pretty_generate({ pending_secret: setup.secret, remember_cookie: cookie }))
    puts "WS9 Rails rollback: #{rows.length} Rust-written rows validated; credential/setup secrets decrypted; session/device/backup/cookie/TOTP checks passed"
  ensure
    travel_back
  end
end

TwoFactorRollback.new.run
