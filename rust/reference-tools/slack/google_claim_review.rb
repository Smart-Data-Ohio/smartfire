# PR #195: cases outside the original opt-in interaction. Loaded by google_claim_http.rb.
review_login = lambda do |browser, subject, email|
  browser.get('/session/new')
  Thread.current[:ws16_hex] = [1, 2].map { |b| [b].pack('C').unpack1('H*') * 16 }
  Thread.current[:ws16_verifier] = 3
  browser.post('/session/google', headers: { 'HTTP_X_CSRF_TOKEN' => browser.request.session[:_csrf_token] })
  q = Rack::Utils.parse_query(URI(browser.response.location).query)
  Thread.current[:ws16_hex] = nil; Thread.current[:ws16_verifier] = nil
  claims = { 'iss' => 'https://accounts.google.com', 'aud' => 'test-client-id', 'sub' => subject,
    'email' => email, 'email_verified' => true, 'hd' => 'smartdata.net', 'name' => 'Alice',
    'nonce' => q.fetch('nonce'), 'exp' => Time.current.to_i + 3600, 'auth_time' => Time.current.to_i }
  $id_token = JWT.encode(claims, KEY, 'RS256', { kid: 'fixture' })
  browser.get('/session/google/callback', params: { state: q.fetch('state'), code: 'fixture-code' })
end
review_response = lambda do |browser|
  { status: browser.response.status, location: browser.response.location,
    content_type: browser.response.headers['Content-Type'], body: browser.response.body }
end
review_claims = [
  { name: 'unicode_sigma', placeholder: 'οσ@smartdata.net', email: 'ΟΣ@smartdata.net' },
  { name: 'unicode_other_user', placeholder: 'οσ@smartdata.net', email: 'ΟΣ@smartdata.net', rival: 'ος@smartdata.net' },
  { name: 'unicode_predecessor', placeholder: 'οσ-deactivated-review@smartdata.net', email: 'ΟΣ@smartdata.net', deactivated: true }
].map do |input|
  floor = User.maximum(:id)
  user = User.new(name: 'Sigma placeholder', email_address: input.fetch(:placeholder),
    google_email_link_allowed: true, status: input[:deactivated] ? :deactivated : :active)
  user.skip_open_room_grant = true if input[:deactivated]
  user.save!
  rival = User.create!(name: 'Sigma administrator', email_address: input[:rival],
    role: :administrator, google_email_link_allowed: true) if input[:rival]
  browser = ActionDispatch::Integration::Session.new(Rails.application); browser.host! 'campfire.test'
  subject = "review-#{input.fetch(:name)}"
  review_login.call(browser, subject, input.fetch(:email))
  identity = GoogleIdentity.find_by(subject:)
  owner = if identity.nil? then 'none'
    elsif identity.user_id == user.id then 'placeholder'
    elsif identity.user_id == rival&.id then 'administrator'
    else 'new_user' end
  state = db.select_all("SELECT users.name,users.email_address,users.role,users.status,google_identities.subject FROM users LEFT JOIN google_identities ON google_identities.user_id=users.id WHERE users.id>#{floor} ORDER BY users.id").to_a
  result = input.merge(response: review_response.call(browser), owner:, users: state,
    sessions: Session.where(user_id: User.where('id > ?', floor).select(:id)).count)
  ids = User.where('id > ?', floor).pluck(:id)
  Session.where(user_id: ids).delete_all
  GoogleIdentity.where(user_id: ids).delete_all
  Membership.where(user_id: ids).delete_all
  User.where(id: ids).delete_all
  result
end
review_preview_queries = [10, 200].map do |size|
  members = size.times.map { |i| { 'id' => "UREVIEW#{size}_#{i}", 'profile' => { 'email' => "preview-#{size}-#{i}@smartdata.net" } } }
  sql = []
  observer = lambda do |*args|
    payload = args.last
    sql << payload[:sql] if payload[:sql].match?(/\ASELECT\b/i) && !payload[:cached]
  end
  delta = ActiveSupport::Notifications.subscribed(observer, 'sql.active_record') do
    ActiveRecord::Base.uncached { Slack::UserMapper.new(workspace:, run:).preview_page(members) }
  end
  { size:, selects: sql.length, sql:, delta: }
end
# The previous interaction's response snapshots are already captured. Avoid reusing its
# deterministic backup codes across credentials (Rails validates global digest uniqueness).
TwoFactorBackupCode.delete_all
review_enrollment_user = User.create!(name: 'Enrollment review', email_address: 'enrollment-review@smartdata.net', google_email_link_allowed: true)
review_browser = ActionDispatch::Integration::Session.new(Rails.application); review_browser.host! 'campfire.test'
review_login.call(review_browser, 'review-enrollment', review_enrollment_user.email_address)
review_browser.get('/two_factor_setup')
review_secret = TwoFactorSetupSecret.order(:id).last
review_code = ROTP::TOTP.new(review_secret.secret).at(Time.current)
db.execute("CREATE TRIGGER refuse_review_enable BEFORE INSERT ON audit_logs WHEN NEW.action='two_factor.enable' BEGIN SELECT RAISE(ABORT,'review audit unavailable'); END")
review_browser.post('/two_factor_setup', params: { code: review_code }, headers: { 'HTTP_X_CSRF_TOKEN' => review_browser.request.session[:_csrf_token] })
review_enrollment_failure = { response: review_response.call(review_browser), state: {
  enrolled: review_enrollment_user.reload.two_factor_enabled?,
  backups: TwoFactorBackupCode.joins(:two_factor_credential).where(two_factor_credentials: { user_id: review_enrollment_user.id }).count,
  pending_secrets: TwoFactorSetupSecret.joins(:session).where(sessions: { user_id: review_enrollment_user.id }).count,
  verified_sessions: Session.where(user_id: review_enrollment_user.id).where.not(two_factor_verified_at: nil).count,
  sessions: Session.where(user_id: review_enrollment_user.id).count,
  audits: AuditLog.where(action: 'two_factor.enable', target_id: review_enrollment_user.id).count
} }
review_browser.get('/two_factor_setup')
review_enrollment_failure[:after] = review_response.call(review_browser)
db.execute('DROP TRIGGER refuse_review_enable')
review_results = { claims: review_claims, preview_queries: review_preview_queries, enrollment_audit_failure: review_enrollment_failure }
puts "Google claim review oracle: #{review_claims.length} signed callback cases; preview SELECTs #{review_preview_queries.map { |r| "#{r[:size]}=#{r[:selects]}" }.join(', ')}; enrollment audit failure #{review_enrollment_failure[:state]}"
review_results
