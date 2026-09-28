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
  # A queued run whose step job was enqueued within this window is left
  # alone: its job is still pending. Past the window the job is presumed
  # lost and the sweeper enqueues again.
  ENQUEUE_COOLDOWN = 5.minutes

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
      kind: kind, mode: mode, options: normalized,
      state: { "enqueued_at" => Time.current.iso8601(6) }).tap do |run|
      SlackImport::StepJob.perform_later(run.id)
    end
  end

  # Runs holding a fresh step lease: a step or undo job recorded
  # step_started_at when it started and clears it when it ends, so a
  # fresh stamp means the job may still be executing even when the
  # status already flipped underneath it (cancelled or failed).
  scope :with_fresh_lease, -> {
    where("json_extract(state, '$.step_started_at') > ?", STALE_HEARTBEAT.ago.iso8601(6))
  }
  # Runs that block new claims: a step or undo job may still be
  # executing for them — running or undoing by status, or holding a
  # fresh step lease.
  scope :claim_blocking, -> { where(status: %w[ running undoing ]).or(with_fresh_lease) }
  # Runs that block an undo claim: another run still queued, running
  # or undoing, or holding a fresh step lease. Separate from
  # claim_blocking: the step claim and the kick must not block on a
  # queued status, or the claimant would block itself.
  scope :undo_blocking, -> { where(status: %w[ queued running undoing ]).or(with_fresh_lease) }

  # Single-flight claim: flips one queued run to running only when no
  # other run is running or undoing and none holds a fresh step lease,
  # in a single conditional UPDATE. Imports and undos never interleave.
  # Returns true when this run won the claim.
  def self.claim_running!(id)
    now = Time.current
    where(id: id, status: "queued")
      .where("NOT EXISTS (?)", claim_blocking.select("1"))
      .update_all(status: "running", started_at: now, heartbeat_at: now,
        updated_at: now) == 1
  end

  # Starts the oldest queued run when nothing is running or undoing, no
  # run holds a fresh step lease, and no job is already pending for it.
  # Called whenever a run finishes and by the periodic sweeper, so queued
  # runs hand off promptly without ever stacking a second job behind one
  # already pending.
  def self.kick_next_queued!
    return if claim_blocking.exists?

    oldest = where(status: "queued").order(:created_at, :id).first
    return if oldest.nil? || oldest.step_job_pending?

    oldest.enqueue_step_job!
  end

  # Periodic sweeper (see Periodic::Runner): re-enqueues running runs whose
  # heartbeat went stale, starts the oldest queued run when nothing is
  # running or undoing, and re-enqueues undoing runs that stall.
  def self.sweep_stalled!
    stale = STALE_HEARTBEAT.ago
    where(status: "running").where("heartbeat_at IS NULL OR heartbeat_at < ?", stale)
      .find_each { |run| SlackImport::StepJob.perform_later(run.id) }

    kick_next_queued!

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

    Time.iso8601(value.to_s).iso8601(6)
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
    self.class.kick_next_queued!
    true
  end

  # Imports (not dry runs) that stopped can be undone: everything the run
  # created is removed; records it only matched are left alone. Undo is
  # last-in, first-out per conversation: a later import that touched any of
  # the same conversations skipped this run's messages as already mapped,
  # so undoing this one first would leave a hole the later run's coverage
  # hides from every catch-up. That later run must be undone first.
  def undoable?
    undo_eligible? && later_overlapping_import.nil?
  end

  # Why this run cannot be undone right now, or nil when undo may proceed
  # (or the run is not an undoable kind at all). Undo never interleaves
  # with another queued, running or undoing run, or with a run whose
  # step job may still be executing under a fresh step lease.
  def undo_blocked_reason
    return nil unless undo_eligible?

    if (later = later_overlapping_import)
      if later.user_id == user_id
        "A later import (##{later.id}) also imported some of these conversations; undo that one first."
      else
        "A later import by #{later.user.name} also imported some of these conversations. " \
          "It has to be undone first; ask them or an administrator."
      end
    elsif step_lease_fresh?
      "This import is still finishing. Wait for it to finish, then undo."
    elsif self.class.where(status: %w[ queued running undoing ]).where.not(id: id).exists?
      "Another import is queued or running. Wait for it to finish, then undo."
    elsif self.class.with_fresh_lease.where.not(id: id).exists?
      "Another import is still finishing. Wait for it to finish, then undo."
    end
  end

  # The Slack conversations this run wrote into: every conversation whose
  # target resolved to a room (created or merged, including a room an
  # earlier run's mapping pointed at). Skipped and never-reached
  # conversations touched nothing.
  def touched_conversation_ids
    Array(stats["conversations"]).filter_map do |entry|
      entry["id"] if entry.dig("target", "action").in?(%w[ create merge ])
    end
  end

  # Returns false, with no status change, when the run can't be undone or
  # another run is queued, running or undoing, or holds a fresh step
  # lease. The claim is a single conditional UPDATE, so two undos racing
  # each other still serialize, and a new run (which always starts
  # queued) can't slip in between the later-import check and the claim.
  def undo!
    return false unless undoable?
    return false if undo_blocked_reason

    now = Time.current
    stats = self.stats.merge("phase" => "undo")
    claimed = self.class.where(id: id, status: %w[ completed failed cancelled ])
      .where("NOT EXISTS (?)",
        self.class.undo_blocking.where.not(id: id).select("1"))
      .update_all(status: "undoing", state: { "phase" => "undo" }, stats: stats,
        heartbeat_at: now, finished_at: nil, updated_at: now) == 1
    return false unless claimed

    self.status = "undoing"
    self.state = { "phase" => "undo" }
    self.stats = stats
    self.heartbeat_at = now
    self.finished_at = nil
    SlackImport::UndoJob.perform_later(id)
    true
  end

  # Whether a step job is already pending for this queued run. A job that
  # ran but failed to claim its run clears the stamp (see StepJob), so a
  # fresh stamp always means a job still waiting in the queue.
  def step_job_pending?
    enqueued_at = state["enqueued_at"]
    enqueued_at.present? && Time.iso8601(enqueued_at.to_s) > ENQUEUE_COOLDOWN.ago
  rescue ArgumentError, Date::Error
    false
  end

  # Enqueues this queued run's step job, stamping it so the sweeper and
  # later kicks never stack a second job behind the pending one.
  def enqueue_step_job!
    update!(state: state.merge("enqueued_at" => Time.current.iso8601(6)))
    SlackImport::StepJob.perform_later(id)
  end

  # The pending job ran but lost its claim, so it is no longer pending:
  # the next finish or sweeper tick enqueues a fresh one.
  def clear_pending_step_job!
    return unless state["enqueued_at"].present?

    update!(state: state.except("enqueued_at"))
  end

  # Records the step lease: the run is busy while a step or undo job
  # executes. Conditional on the expected status, so a job whose run was
  # cancelled underneath it never starts writing. Returns false, writing
  # nothing, when the run already left that status.
  def acquire_step_lease!(expected_status)
    now = Time.current
    merged = state.merge("step_started_at" => now.iso8601(6))
    claimed = self.class.where(id: id, status: expected_status)
      .update_all(state: merged, heartbeat_at: now, updated_at: now) == 1
    if claimed
      self.state = merged
      self.heartbeat_at = now
    end
    claimed
  end

  # Clears the step lease. Reads the row's current state (the step saved
  # progress since acquiring) and drops only the lease key.
  def release_step_lease!
    current = self.class.where(id: id).pick(:state) || {}
    return unless current["step_started_at"].present?

    self.class.where(id: id).update_all(
      state: current.except("step_started_at"), updated_at: Time.current)
    self.state = state.except("step_started_at")
  end

  # Whether a step or undo job may still be executing for this run: it
  # recorded step_started_at when it started and clears it when it ends.
  # Stale past the heartbeat window, since a crashed job never clears it.
  def step_lease_fresh?
    started_at = state["step_started_at"]
    started_at.present? && Time.iso8601(started_at.to_s) > STALE_HEARTBEAT.ago
  rescue ArgumentError, Date::Error
    false
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
    self.class.kick_next_queued!
  end

  # Reloading drops the memoized later-overlap scan with the attributes.
  def reload(*)
    remove_instance_variable(:@later_overlapping_import) if defined?(@later_overlapping_import)
    super
  end

  private
    def undo_eligible?
      import? && (completed? || failed? || cancelled?)
    end

    # The most recent import that started after this one, is not undone, and
    # touched any conversation this run touched (from the runs' per-
    # conversation stats, plus this run's own conversation mappings, which
    # cover a crash between creating a room and saving its target).
    # Memoized per instance: undo_blocked_reason, undoable? and undo!
    # share one scan within a request.
    def later_overlapping_import
      return @later_overlapping_import if defined?(@later_overlapping_import)
      @later_overlapping_import = find_later_overlapping_import
    end

    def find_later_overlapping_import
      return nil if started_at.nil?

      mine = (touched_conversation_ids +
        records.where(slack_kind: "conversation").pluck(:slack_key)).to_set
      return nil if mine.empty?

      self.class.where(slack_workspace_id:, mode: "import").where.not(id:).where.not(status: "undone")
        .where("started_at > :at OR (started_at = :at AND id > :id)", at: started_at, id:)
        .order(started_at: :desc, id: :desc).select(:id, :user_id, :stats)
        .detect { |run| run.touched_conversation_ids.any? { |conversation_id| mine.include?(conversation_id) } }
    end
end
