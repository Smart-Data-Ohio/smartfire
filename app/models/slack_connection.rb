# A member's Slack user token, granted through the internal app's OAuth
# flow. An administrator's connection drives the workspace import; any
# member's drives a personal import of their own DMs, group DMs and private
# channels. The token is encrypted at rest and never logged or rendered.
class SlackConnection < ApplicationRecord
  belongs_to :slack_workspace
  belongs_to :user

  has_many :imports, class_name: "SlackImport", dependent: :nullify

  encrypts :access_token

  validates :slack_user_id, presence: true

  def connected?
    disconnected_reason.blank? && access_token.present?
  end
end
