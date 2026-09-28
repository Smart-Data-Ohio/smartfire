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
    create!(slack_workspace: workspace, user: user, slack_connection: connection,
      kind: kind, mode: mode, options: options.to_h.stringify_keys)
  end

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
    update!(status: "undoing")
    true
  end

  def finished?
    !active?
  end
end
