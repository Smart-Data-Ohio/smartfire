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

  def active?
    status.in?(ACTIVE_STATUSES)
  end

  def finished?
    !active?
  end
end
