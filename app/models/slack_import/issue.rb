# A warning or error an import run hit on one Slack object, shown on the
# run's page (for example a message that failed to convert).
class SlackImport::Issue < ApplicationRecord
  self.table_name = "slack_import_issues"

  LEVELS = %w[ warning error ].freeze

  belongs_to :slack_import

  enum :level, LEVELS.index_by(&:itself)

  validates :message, presence: true
end
