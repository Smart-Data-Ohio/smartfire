module User::Starring
  extend ActiveSupport::Concern

  included do
    has_many :user_stars, dependent: :delete_all
    has_many :starred_users, through: :user_stars, source: :starred_user
    has_many :received_stars, class_name: "UserStar", foreign_key: :starred_user_id, dependent: :delete_all
  end

  def starred?(other)
    user_stars.where(starred_user_id: other.id).exists?
  end

  # The ids in the given list this user starred, for per-viewer flags.
  def starred_ids_among(user_ids)
    user_stars.where(starred_user_id: user_ids).pluck(:starred_user_id).to_set
  end
end
