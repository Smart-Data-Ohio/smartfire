class DndAllowedUser < ApplicationRecord
  belongs_to :user
  belongs_to :allowed_user, class_name: "User"

  validates :allowed_user_id, uniqueness: { scope: :user_id }
  validate :allowed_user_must_be_someone_else
  validate :allowed_user_must_be_active_human

  private
    def allowed_user_must_be_someone_else
      errors.add(:allowed_user, "must be someone else") if user_id.present? && user_id == allowed_user_id
    end

    def allowed_user_must_be_active_human
      person = allowed_user
      return if person.nil?

      unless person.active? && !person.bot?
        errors.add(:allowed_user, "must be an active person")
      end
    end
end
