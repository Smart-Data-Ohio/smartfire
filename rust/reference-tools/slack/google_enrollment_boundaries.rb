# Loaded by google_claim_http.rb: actual signed Google login, TOTP and CSRF.
boundary_faults = [
  ['credential_insert', false, "BEFORE INSERT ON two_factor_credentials WHEN NEW.user_id={user}"],
  ['confirm_update', false, "BEFORE UPDATE ON two_factor_credentials WHEN NEW.user_id={user} AND NEW.confirmed_at IS NOT NULL"],
  ['setup_delete', false, "BEFORE DELETE ON two_factor_setup_secrets WHEN OLD.session_id={current}"],
  ['backup_delete', true, "BEFORE DELETE ON two_factor_backup_codes WHEN OLD.two_factor_credential_id=(SELECT id FROM two_factor_credentials WHERE user_id={user})"],
  ['backup_insert', false, "BEFORE INSERT ON two_factor_backup_codes WHEN NEW.two_factor_credential_id=(SELECT id FROM two_factor_credentials WHERE user_id={user}) AND (SELECT COUNT(*) FROM two_factor_backup_codes WHERE two_factor_credential_id=NEW.two_factor_credential_id)>=3"],
  ['backup_replace_insert', true, "BEFORE INSERT ON two_factor_backup_codes WHEN NEW.two_factor_credential_id=(SELECT id FROM two_factor_credentials WHERE user_id={user}) AND (SELECT COUNT(*) FROM two_factor_backup_codes WHERE two_factor_credential_id=NEW.two_factor_credential_id)>=3"],
  ['session_verify', false, "BEFORE UPDATE ON sessions WHEN NEW.id={current} AND NEW.two_factor_verified_at IS NOT NULL"],
  ['session_destroy_first', false, "BEFORE DELETE ON sessions WHEN OLD.user_id={user} AND OLD.device_id='other-1'"],
  ['session_destroy_second', false, "BEFORE DELETE ON sessions WHEN OLD.user_id={user} AND OLD.device_id='other-2'"],
  ['session_setup_delete_second', false, "BEFORE DELETE ON two_factor_setup_secrets WHEN OLD.session_id IN (SELECT id FROM sessions WHERE user_id={user} AND device_id='other-2')"],
  ['audit', false, "BEFORE INSERT ON audit_logs WHEN NEW.action='two_factor.enable' AND NEW.target_id={user}"],
  ['success', false, nil],
  ['wrong_code', false, nil]
]
boundary_stage = lambda do |sql|
  text = sql.delete('"').upcase
  { 'INSERT INTO TWO_FACTOR_CREDENTIALS' => 'credential.insert',
    'UPDATE TWO_FACTOR_CREDENTIALS' => 'credential.confirm',
    'DELETE FROM TWO_FACTOR_SETUP_SECRETS' => 'setup.delete',
    'UPDATE TWO_FACTOR_SETUP_SECRETS' => 'setup.refresh',
    'DELETE FROM TWO_FACTOR_BACKUP_CODES' => 'backup.delete',
    'INSERT INTO TWO_FACTOR_BACKUP_CODES' => 'backup.insert',
    'UPDATE SESSIONS' => 'session.verify', 'DELETE FROM SESSIONS' => 'session.destroy',
    'DELETE FROM WORKSPACE_PRESENCE_LEASES' => 'presence.delete',
    'INSERT INTO AUDIT_LOGS' => 'audit.insert' }.find { |prefix, _| text.start_with?(prefix) }&.last
end
boundary_cases = boundary_faults.each_with_index.map do |(name, existing, fault), index|
  # Each scenario starts with an independent app, as the Rust fixture does.
  Rails.cache.clear
  TwoFactor::SetupsController.cache_store.clear
  TwoFactorBackupCode.delete_all
  backup_index = 0
  ids = { user: 9_000_000_100 + index, current: 9_000_000_200 + index * 3 }
  { users: ids[:user]-1, sessions: ids[:current]-1, two_factor_credentials: 9_000_000_300+index-1,
    two_factor_setup_secrets: 9_000_000_400+index*3-1 }.each do |table, seq|
    db.execute("UPDATE sqlite_sequence SET seq=#{seq} WHERE name='#{table}'")
  end
  user = User.create!(name: 'Enrollment boundary', email_address: "boundary-#{name}@smartdata.net", google_email_link_allowed: true)
  raise 'fixture user id' unless user.id == ids[:user]
  credential = TwoFactorCredential.create!(user:, secret: 'KRUGS4ZANFZSAYJA') if existing
  %w[oldcodeone oldcodetwo].each { |code| TwoFactorBackupCode.create!(two_factor_credential: credential, code_digest: TwoFactorBackupCode.digest(code)) } if existing
  browser = ActionDispatch::Integration::Session.new(Rails.application); browser.host! 'campfire.test'
  review_login.call(browser, "boundary-#{name}", user.email_address)
  current = user.sessions.order(:id).last
  raise 'fixture session id' unless current.id == ids[:current]
  browser.get('/two_factor_setup')
  %w[other-1 other-2].each do |device_id|
    other = Session.create!(user:, device_id:, last_active_at: Time.current)
    TwoFactorSetupSecret.issue_for!(other)
  end
  trigger = fault && "CREATE TRIGGER boundary_fail #{fault} BEGIN SELECT RAISE(ABORT,'boundary unavailable'); END"
  trigger = trigger&.gsub('{user}', ids[:user].to_s)&.gsub('{current}', ids[:current].to_s)
  db.execute(trigger) if trigger
  code = name == 'wrong_code' ? 'invalid' : ROTP::TOTP.new(TwoFactorSetupSecret.find_by!(session: current).secret).at(Time.current)
  groups = []; pending = nil
  observer = lambda do |*args|
    sql = args.last[:sql]; upper = sql.upcase
    if upper.start_with?('BEGIN')
      pending = []
    elsif upper.start_with?('COMMIT', 'ROLLBACK')
      groups << { outcome: upper.start_with?('COMMIT') ? 'commit' : 'rollback', writes: pending } if pending&.any?
      pending = nil
    elsif (stage = boundary_stage.call(sql))
      raise "write without transaction: #{sql}" unless pending
      pending << stage
    end
  end
  ActiveSupport::Notifications.subscribed(observer, 'sql.active_record') do
    browser.post('/two_factor_setup', params: { code: }, headers: { 'HTTP_X_CSRF_TOKEN' => browser.request.session[:_csrf_token] })
  end
  credential = user.reload.two_factor_credential
  label = "CASE WHEN sessions.id=#{current.id} THEN 'current' ELSE sessions.device_id END"
  state = {
    credentials: db.select_all("SELECT confirmed_at,last_totp_at,created_at,updated_at FROM two_factor_credentials WHERE user_id=#{user.id}").to_a,
    decryption: credential && { secret_present: credential.secret.present?,
      setup_secret_matches: credential.enabled? ? credential.secret == 'JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP' : nil,
      prior_secret_preserved: existing ? credential.secret == 'KRUGS4ZANFZSAYJA' : nil },
    backups: db.select_all("SELECT code_digest,used_at FROM two_factor_backup_codes WHERE two_factor_credential_id IN (SELECT id FROM two_factor_credentials WHERE user_id=#{user.id}) ORDER BY code_digest").to_a,
    sessions: db.select_all("SELECT #{label} AS label,two_factor_verified_at,last_active_at FROM sessions WHERE user_id=#{user.id} ORDER BY sessions.id").to_a,
    pending: db.select_all("SELECT #{label} AS label,expires_at FROM two_factor_setup_secrets JOIN sessions ON sessions.id=two_factor_setup_secrets.session_id WHERE user_id=#{user.id} ORDER BY sessions.id").to_a,
    audits: AuditLog.where(action: 'two_factor.enable', target_id: user.id).map(&:details)
  }
  result = { name:, existing:, user_id: user.id, current_id: current.id,
    credential_seq: 9_000_000_300+index-1, setup_seq: 9_000_000_400+index*3-1,
    trigger: fault && "CREATE TRIGGER boundary_fail #{fault} BEGIN SELECT RAISE(ABORT,'boundary unavailable'); END",
    response: review_response.call(browser), state:, transactions: groups }
  db.execute('DROP TRIGGER boundary_fail') if trigger
  browser.get('/two_factor_setup')
  result[:after] = review_response.call(browser)
  puts "Rails enrollment boundary #{name}: status=#{result[:response][:status]}; credentials=#{state[:credentials].length}; confirmed=#{credential&.enabled? || false}; backups=#{state[:backups].length}; pending=#{state[:pending].length}; verified=#{state[:sessions].count { |row| row['two_factor_verified_at'] }}; sessions=#{state[:sessions].length}; write transactions=#{groups.length}"
  TwoFactorSetupSecret.where(session_id: (current.id..current.id+2)).delete_all
  Session.where(user_id: user.id).delete_all
  TwoFactorBackupCode.where(two_factor_credential_id: credential&.id).delete_all
  TwoFactorCredential.where(user_id: user.id).delete_all
  GoogleIdentity.where(user_id: user.id).delete_all
  Membership.where(user_id: user.id).delete_all
  User.where(id: user.id).delete_all
  result
end
boundary_sources = %w[app/controllers/two_factor/setups_controller.rb app/models/two_factor_credential.rb app/models/two_factor_backup_code.rb app/models/two_factor_setup_secret.rb app/models/session.rb app/controllers/concerns/authentication.rb]
boundary_output = { reference: ENV.fetch("PARITY_REFERENCE_SHA"), sources: boundary_sources.to_h { |path| [path, Digest::SHA256.file(Rails.root.join(path)).hexdigest] }, cases: boundary_cases }
File.write(File.join(WORK,'vectors/slack/google_enrollment_boundaries.json'), JSON.pretty_generate(boundary_output)+"\n")
puts "Rails enrollment boundary oracle: #{boundary_cases.length} cases; literal HTTP responses, retained rows and write transaction groups"
