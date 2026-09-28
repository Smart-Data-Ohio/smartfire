# The Slack workspace being migrated, and the internal Slack app an admin
# created for it (docs/slack-import.md). One row per Smartfire account.
# team_* fill in from the first successful OAuth connection.
class SlackWorkspace < ApplicationRecord
  belongs_to :configured_by, class_name: "User", optional: true

  has_many :connections, class_name: "SlackConnection", dependent: :destroy
  has_many :imports, class_name: "SlackImport", dependent: :destroy
  has_many :import_records, class_name: "SlackImport::Record", dependent: :delete_all

  encrypts :client_secret

  validates :client_id, :client_secret, presence: true

  def self.current
    first
  end

  def app_configured?
    client_id.present? && client_secret.present?
  end
end
