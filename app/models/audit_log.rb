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
    room.create
    room.destroy
    room.membership.change
    workspace_icon.create
    workspace_icon.destroy
  ].freeze

  # Sign-in failures are recorded at most once per IP per window, so a
  # credential-stuffing flood leaves one row instead of thousands.
  SIGN_IN_FAILURE_THROTTLE_WINDOW = 5.minutes

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
  #     changes: { role: %w[ member administrator ] })
  #
  # changes values SHOULD be [ before, after ] pairs; scalar context values
  # (reasons, urls, notes) are allowed. Everything in changes passes through
  # the secret filter before it is stored.
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
      user_agent: user_agent || request&.user_agent
    )
  end

  # A sign-in failure, throttled per IP so floods do not spam the log.
  # Returns the row, or nil when a row for this IP already exists inside
  # the window. Successes are never throttled: each one proves credentials.
  def self.record_sign_in_failure!(email:, method:, request: nil)
    request ||= Current.request
    ip = request&.remote_ip

    if ip.present? && where(action: "session.sign_in.failure", ip_address: ip)
        .where(created_at: SIGN_IN_FAILURE_THROTTLE_WINDOW.ago..).exists?
      return nil
    end

    record!(action: "session.sign_in.failure", actor_label: email.presence,
      changes: { method: method }, request: request)
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
