# One user's private "float to the top" mark on another user. Stars drive
# the member panel's Starred group and the starred-first ordering in the
# people directory and the new-DM picker. They are never shown to the
# starred person, and deactivated users keep their rows while dropping
# out of every display.
class UserStar < ApplicationRecord
  belongs_to :user
  belongs_to :starred_user, class_name: "User"

  validates :starred_user_id, uniqueness: { scope: :user_id }
  validate :starred_user_must_be_someone_else

  private
    def starred_user_must_be_someone_else
      errors.add(:starred_user, "must be someone else") if user_id.present? && user_id == starred_user_id
    end
end
