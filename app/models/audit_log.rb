# Append-only log of security-relevant actions, browsable by administrators
# at /account/audit_log. See docs/audit-log.md for the action vocabulary and
# how to record new actions.
#
# Every write funnels through .record!: controllers, services, and jobs call
# it with an action name, an actor, a target, and a changes hash. Actor and
# target rows may later be deleted or renamed, so record! snapshots a label
# for each at write time and stores no foreign keys.
#
# Append-only is enforced here: persisted rows are readonly and destroy is
# refused. The only delete path in the app is the retention prune
# (Retention::PruneJob), which removes rows older than a year with delete_all.
class AuditLog < ApplicationRecord
  # Known actions, grouped by area. The admin filter dropdown lists these;
  # rows always store the plain string so new actions need no migration.
  ACTIONS = %w[
    session.sign_in.success
    session.sign_in.failure
    sign_in.two_factor.failure
    two_factor.enable
    two_factor.disable
    two_factor.reset
    two_factor.backup_codes.regenerate
    user.create
    user.email.change
    user.password.change
    user.role.change
    user.ban
    user.unban
    user.deactivate
    google.sign_in.link
    google.sign_in.link_allow
    google.sign_in.unlink
    google.account.connect
    google.account.disconnect
    github.account.connect
    github.account.disconnect
    fizzy.account.connect
    fizzy.account.disconnect
    account.join_code.reset
    account.settings.change
    account.custom_styles.change
    agent.create
    agent.update
    agent.suspend
    agent.credential.create
    agent.credential.revoke
    agent.credential.reset
    agent.grant.create
    agent.grant.revoke
    agent.webhook_url.change
    agent.webhook_secret.reset
    agent.github.connect
    agent.github.disconnect
    agent.approval.decide
    agent.github_action.execute
    agent.fizzy_action.execute
    agent.kill_switch
    room.create
    room.destroy
    room.membership.change
    board.automation.change
    work.handoff
    workspace_icon.create
    workspace_icon.destroy
  ].freeze

  # Sign-in failures collapse per (IP, email label) inside the window,
  # so retrying one address leaves one row while a spray across
  # addresses still shows each target. Past SIGN_IN_FAILURES_PER_IP_WINDOW
  # rows from one IP inside the window, further failures only bump the
  # latest row's suppressed_count instead of inserting.
  SIGN_IN_FAILURE_THROTTLE_WINDOW = 5.minutes
  SIGN_IN_FAILURES_PER_IP_WINDOW = 20

  # A failed sign-in stores the typed email only when it matches a real
  # account or has an email shape; anything else is likely a password
  # typed into the email field, which must not be kept for a year.
  UNRECOGNIZED_ACTOR_LABEL = "[unrecognized]".freeze
  FAILURE_LABEL_MAX = 254
  # Longest input the email-shape check examines. Real emails top out at
  # 254 chars; anything past this is a pasted password or an attack, and
  # is never stored as a label (see failure_actor_label).
  FAILURE_EMAIL_SHAPE_MAX = 1000

  # Keys whose values are secrets and must never land in the log. Callers
  # pass explicit change hashes (never raw params), and this filter is the
  # backstop. Deliberately narrower than the request log filter: actor and
  # target emails stay readable in the log.
  SECRET_KEY_PATTERN = /passw|passwd|pwd|secret|token|api[-_]?key|_key\z|credential|authorization|cookie|session|join[-_]?code|transfer[-_]?id/i
  FILTERED = "[FILTERED]".freeze

  validates :action, presence: true

  # The single write path. Actor and request default to the current request
  # (nil outside one, e.g. jobs and console); pass actor: nil explicitly is
  # unnecessary since Current.user is already nil there.
  #
  #   AuditLog.record!(action: "user.role.change", target: user,
  #     changes: { role: AuditLog.pair("member", "administrator") })
  #
  # changes values SHOULD be AuditLog.pair(before, after) pairs; scalar
  # context values (reasons, urls, notes) are allowed. Everything in
  # changes passes through the secret filter before it is stored.
  def self.record!(action:, actor: nil, actor_label: nil, target: nil, target_label: nil, changes: nil, request: nil, ip_address: nil, user_agent: nil)
    actor ||= Current.user
    request ||= Current.request

    create!(
      action: action,
      actor_id: actor&.id,
      actor_label: actor_label || label_for(actor),
      target_type: target&.class&.base_class&.name,
      target_id: target.try(:id),
      target_label: target_label || label_for(target),
      details: filter_secrets(changes),
      ip_address: ip_address || request&.remote_ip,
      user_agent: (user_agent || request&.user_agent)&.truncate(USER_AGENT_MAX)
    )
  end

  # A sign-in failure, collapsed per (IP, email label) so floods do not
  # spam the log. Returns the row, or nil when this IP already failed for
  # this label inside the window. Past the per-IP cap the latest row's
  # suppressed_count is bumped and that row returned. Successes are never
  # throttled: each one proves credentials.
  def self.record_sign_in_failure!(email:, method:, request: nil)
    request ||= Current.request
    ip = request&.remote_ip
    label = failure_actor_label(email)

    if ip.present?
      window = where(action: "session.sign_in.failure", ip_address: ip)
        .where(created_at: SIGN_IN_FAILURE_THROTTLE_WINDOW.ago..)
      return nil if window.where("LOWER(actor_label) = ?", label.downcase).exists?
      return increment_suppressed_failures!(window) if window.count >= SIGN_IN_FAILURES_PER_IP_WINDOW
    end

    record!(action: "session.sign_in.failure", actor_label: label,
      changes: { method: method }, request: request)
  end

  # The IP already filled its window: fold this failure into the latest
  # row's suppressed_count with a direct update (rows are otherwise
  # append-only) and return that row.
  def self.increment_suppressed_failures!(window)
    latest = window.order(id: :desc).first
    return nil if latest.nil?

    details = latest.details || {}
    count = details["suppressed_count"].to_i + 1
    where(id: latest.id).update_all(details: details.merge("suppressed_count" => count))
    latest.reload
  end

  # The actor label for a failed sign-in: the typed value, truncated,
  # when it matches an existing account (case-insensitively, like
  # sign-in itself) or has an email shape; "[unrecognized]" otherwise,
  # so a password typed into the email field is never stored.
  def self.failure_actor_label(email)
    typed = email.to_s.strip
    return UNRECOGNIZED_ACTOR_LABEL if typed.blank?

    known = failure_email_shape?(typed) ||
      User.where("LOWER(email_address) = ?", typed.downcase).exists?
    known ? typed.truncate(FAILURE_LABEL_MAX) : UNRECOGNIZED_ACTOR_LABEL
  end

  # Linear email-shape check: exactly one "@", a non-empty local part, a
  # domain with a dot that is neither first nor last, and no whitespace.
  # Split-based with no nested or ambiguous quantifiers, so adversarial
  # input with many dots cannot cause backtracking; overlong input fails
  # fast on the length cap before any splitting.
  def self.failure_email_shape?(value)
    return false if value.length > FAILURE_EMAIL_SHAPE_MAX
    return false unless value.count("@") == 1

    local, _, domain = value.partition("@")
    return false if local.empty? || domain.empty?
    return false if domain.start_with?(".") || domain.end_with?(".")
    return false unless domain.include?(".")
    !(local.match?(/\s/) || domain.match?(/\s/))
  end

  # Webhook URLs often carry secrets in the path or query, so the log
  # keeps only the origin (scheme + host + non-default port) plus a
  # digest prefix of the full URL: a change stays visible without the
  # secret. Nil in, nil out, so clearing a URL still logs.
  DIGEST_PREFIX_LENGTH = 12
  USER_AGENT_MAX = 512

  def self.webhook_origin_summary(url)
    return nil if url.blank?

    origin = parse_webhook_origin(url.to_s)
    { origin: origin, digest: Digest::SHA256.hexdigest(url.to_s)[0, DIGEST_PREFIX_LENGTH] }
  end

  # Marks a before/after pair explicitly so the admin UI renders it as
  # "before → after". Plain two-element arrays (name lists, digests)
  # render as lists instead.
  def self.pair(before, after)
    { before: before, after: after }
  end

  def self.label_for(record)
    case record
    when nil then nil
    when User then "#{record.name} <#{record.email_address}>"
    when Agent then record.user ? "Agent #{record.user.name}" : "Agent ##{record.id}"
    when AgentCredential then "#{record.name} (#{record.agent&.user&.name})"
    when AgentGrant then "#{record.capability} grant (#{record.agent&.user&.name})"
    when AgentApproval then "#{record.action} approval ##{record.id}"
    when Room then record.name
    when Account then record.name
    when WorkspaceIcon then ":#{record.name}:"
    else
      record.try(:name) || "#{record.class.name} ##{record.try(:id)}"
    end
  end

  def self.filter_secrets(changes)
    return {} if changes.nil?

    hash = changes.respond_to?(:to_unsafe_h) ? changes.to_unsafe_h : changes
    hash = { value: hash } unless hash.is_a?(Hash)
    deep_filter(hash).as_json
  end

  # Append-only: persisted rows cannot be updated. The retention prune uses
  # delete_all, which runs SQL directly and is unaffected by this.
  def readonly?
    !new_record?
  end

  def destroy
    raise ActiveRecord::ReadOnlyRecord, "#{self.class} is append-only"
  end

  def delete
    raise ActiveRecord::ReadOnlyRecord, "#{self.class} is append-only"
  end

  # Best-effort target lookup for the admin UI. Targets may be deleted long
  # after the row was written; the UI always falls back to target_label.
  def target_record
    return nil if target_type.blank? || target_id.blank?

    target_type.safe_constantize&.find_by(id: target_id)
  end

  def actor_record
    return nil if actor_id.blank?

    User.find_by(id: actor_id)
  end

  def self.parse_webhook_origin(url)
    uri = URI.parse(url.strip)
    return "[invalid]" unless uri.scheme.present? && uri.host.present?

    origin = "#{uri.scheme}://#{uri.host}"
    origin += ":#{uri.port}" if uri.port && uri.port != uri.default_port
    origin
  rescue URI::InvalidURIError
    "[invalid]"
  end
  private_class_method :parse_webhook_origin

  def self.deep_filter(value)
    case value
    when Hash
      value.to_h do |key, entry|
        if key.to_s.match?(SECRET_KEY_PATTERN)
          [ key, FILTERED ]
        else
          [ key, deep_filter(entry) ]
        end
      end
    when Array
      value.map { |entry| deep_filter(entry) }
    else
      value
    end
  end
  private_class_method :deep_filter
end
