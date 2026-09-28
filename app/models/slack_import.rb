# One run of the Slack importer. A dry run reads the selected scope from
# Slack and reports what an import would do without writing any records; an
# import writes them. Workspace runs use an administrator's connection;
# personal runs cover the connected member's own DMs, group DMs and private
# channels. Runs are resumable (state holds the cursor) and undoable (every
# record they create is listed in SlackImport::Record).
class SlackImport < ApplicationRecord
  KINDS = %w[ workspace personal ].freeze
  MODES = %w[ dry_run import ].freeze
  STATUSES = %w[ queued running completed failed cancelled undoing undone ].freeze
  ACTIVE_STATUSES = %w[ queued running undoing ].freeze

  belongs_to :slack_workspace
  belongs_to :slack_connection, optional: true
  belongs_to :user

  has_many :records, class_name: "SlackImport::Record", dependent: :delete_all
  has_many :issues, class_name: "SlackImport::Issue", dependent: :delete_all

  enum :kind, KINDS.index_by(&:itself)
  enum :mode, MODES.index_by(&:itself)
  enum :status, STATUSES.index_by(&:itself)

  scope :active, -> { where(status: ACTIVE_STATUSES) }
  scope :newest_first, -> { order(created_at: :desc, id: :desc) }

  # Runs record at most this many issues; past it a final notice replaces
  # the rest so a noisy workspace cannot flood the issues table.
  ISSUE_CAP = 1000
  # A running (or undoing) run that has not saved progress within this is
  # considered stalled and re-enqueued by the periodic sweeper.
  STALE_HEARTBEAT = 5.minutes

  # Contract used by the controllers. The engine fills in the bodies
  # (normalizing options, enqueueing the step and undo jobs); the
  # signatures, return values and status transitions stay as documented.

  # Creates a queued run and enqueues it. options (string keys):
  #   "conversation_ids" => [Slack conversation ids] or nil for everything
  #                         in scope
  #   "oldest", "latest" => ISO 8601 dates bounding message time, or nil
  #   "include_private"  => workspace runs: also import private channels
  #                         the connected admin is in (default true)
  #   "room_targets"     => { conversation id => room id | "new" | "skip" }
  def self.start!(workspace:, user:, connection:, kind:, mode:, options: {})
    normalized = normalize_options(options.to_h.stringify_keys)
    create!(slack_workspace: workspace, user: user, slack_connection: connection,
      kind: kind, mode: mode, options: normalized).tap do |run|
      SlackImport::StepJob.perform_later(run.id)
    end
  end

  # Single-flight claim: flips one queued run to running only when no other
  # run is running, in a single conditional UPDATE. Returns true when this
  # run won the claim.
  def self.claim_running!(id)
    now = Time.current
    where(id: id, status: "queued")
      .where("NOT EXISTS (?)", where(status: "running").select("1"))
      .update_all(status: "running", started_at: now, heartbeat_at: now,
        updated_at: now) == 1
  end

  # Periodic sweeper (see Periodic::Runner): re-enqueues running runs whose
  # heartbeat went stale, starts the oldest queued run when nothing is
  # running, and re-enqueues undoing runs that stall.
  def self.sweep_stalled!
    stale = STALE_HEARTBEAT.ago
    where(status: "running").where("heartbeat_at IS NULL OR heartbeat_at < ?", stale)
      .find_each { |run| SlackImport::StepJob.perform_later(run.id) }

    unless where(status: "running").exists?
      oldest = where(status: "queued").order(:created_at, :id).first
      SlackImport::StepJob.perform_later(oldest.id) if oldest
    end

    where(status: "undoing").where("heartbeat_at IS NULL OR heartbeat_at < ?", stale)
      .find_each { |run| SlackImport::UndoJob.perform_later(run.id) }
  end

  def self.normalize_options(options)
    normalized = options.slice("conversation_ids", "oldest", "latest",
      "include_private", "room_targets")
    normalized["conversation_ids"] = Array(normalized["conversation_ids"])
      .filter_map { |id| id.to_s.presence }.uniq.presence
    normalized["oldest"] = normalize_time_bound(normalized["oldest"], "oldest")
    normalized["latest"] = normalize_time_bound(normalized["latest"], "latest")
    normalized["include_private"] = normalized.fetch("include_private", true) != false
    targets = normalized["room_targets"]
    normalized["room_targets"] = targets.is_a?(Hash) ? targets.stringify_keys : {}
    normalized
  end

  def self.normalize_time_bound(value, name)
    return if value.blank?

    Time.iso8601(value.to_s).iso8601
  rescue ArgumentError, Date::Error
    raise ArgumentError, "Slack import #{name} bound is not ISO 8601: #{value.inspect}"
  end
  private_class_method :normalize_options, :normalize_time_bound

  def active?
    status.in?(ACTIVE_STATUSES)
  end

  def cancellable?
    queued? || running?
  end

  # Stops the run at its next step boundary. Returns false when the run
  # had already finished.
  def cancel!
    return false unless cancellable?
    update!(status: "cancelled", finished_at: Time.current)
    true
  end

  # Imports (not dry runs) that stopped can be undone: everything the run
  # created is removed; records it only matched are left alone.
  def undoable?
    import? && (completed? || failed? || cancelled?)
  end

  # Returns false when the run can't be undone.
  def undo!
    return false unless undoable?

    now = Time.current
    stats = self.stats.merge("phase" => "undo")
    update!(status: "undoing", state: { "phase" => "undo" }, stats:,
      heartbeat_at: now, finished_at: nil)
    SlackImport::UndoJob.perform_later(id)
    true
  end

  def finished?
    !active?
  end

  def dry_run?
    mode == "dry_run"
  end

  # Records a warning or error issue, capped so a noisy workspace cannot
  # flood the table. Past the cap one final notice replaces the rest.
  def record_issue!(level, slack_ref, message)
    count = issues.count
    if count >= ISSUE_CAP + 1
      return
    elsif count == ISSUE_CAP
      issues.create!(level: "warning", message: "Further issues suppressed (over #{ISSUE_CAP})")
      return
    end

    issues.create!(level:, slack_ref:, message:)
  end

  # Fails the run with a message. The mapping and state stay behind, so a
  # later run continues where this one stopped.
  def mark_failed!(message)
    update!(status: "failed", error: message, finished_at: Time.current)
  end
end
