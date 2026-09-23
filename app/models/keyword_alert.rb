class KeywordAlert < ApplicationRecord
  MAX_PER_USER = 20
  PHRASE_LIMIT = 80

  belongs_to :user

  normalizes :phrase, with: ->(phrase) { phrase.to_s.strip.gsub(/\s+/, " ").presence }

  validates :phrase, presence: true, length: { maximum: PHRASE_LIMIT }
  validate :phrase_must_be_unique_per_user, if: -> { phrase.present? }
  validate :user_must_have_room_for_another, on: :create

  private
    def phrase_must_be_unique_per_user
      duplicate = KeywordAlert.where(user_id: user_id)
        .where("LOWER(phrase) = ?", phrase.downcase)
        .where.not(id: id)
        .exists?

      errors.add(:phrase, "is already in your list") if duplicate
    end

    def user_must_have_room_for_another
      if KeywordAlert.where(user_id: user_id).count >= MAX_PER_USER
        errors.add(:base, "You can watch at most #{MAX_PER_USER} keywords")
      end
    end
end
