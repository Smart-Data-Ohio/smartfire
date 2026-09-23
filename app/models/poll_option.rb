class PollOption < ApplicationRecord
  LABEL_LIMIT = 200

  belongs_to :poll
  has_many :poll_votes, dependent: :destroy

  validates :label, presence: true, length: { maximum: LABEL_LIMIT }
end
