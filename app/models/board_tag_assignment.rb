# One board's "tag X auto-assigns to Y" rule, configured by the board's
# creator or an administrator. Fires when a post gains the tag while it
# has no owner; it never overrides an existing assignment. See
# ChannelThread#apply_board_tag_auto_assign and docs/board-automations.md.
class BoardTagAssignment < ApplicationRecord
  belongs_to :room
  belongs_to :assignee, class_name: "User"
  belongs_to :created_by, class_name: "User"

  validates :tag, presence: true, length: { maximum: ThreadTag::NAME_LIMIT },
    format: { with: ThreadTag::NAME_FORMAT },
    uniqueness: { scope: :room_id, case_sensitive: false }
  validate :room_must_be_board
  validate :assignee_must_be_eligible

  before_validation :normalize_tag

  private
    def normalize_tag
      self.tag = tag.to_s.strip.downcase
    end

    def room_must_be_board
      errors.add(:room, "must be a board") if room && !room.board?
    end

    # The assignee is whoever may own a board post: an active member, or
    # an active agent member allowed to post there. Grants are re-checked
    # when the rule fires, so a later revocation silently skips the rule.
    def assignee_must_be_eligible
      return if assignee.blank? || room.blank?

      eligible = if assignee.bot?
        agent = assignee.agent || Agent.find_by(user_id: assignee.id)
        assignee.active? && room.memberships.exists?(user_id: assignee.id) &&
          agent&.active? && agent.can?(:post_messages, room)
      else
        assignee.active? && room.memberships.exists?(user_id: assignee.id)
      end

      unless eligible
        errors.add(:assignee, "must be an active board member able to own posts")
      end
    end
end
