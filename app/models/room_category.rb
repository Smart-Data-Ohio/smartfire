class RoomCategory < ApplicationRecord
  NAME_LIMIT = 50

  belongs_to :user
  has_many :memberships, dependent: :nullify

  validates :name, presence: true, length: { maximum: NAME_LIMIT }

  scope :ordered, -> { order(:position, :id) }

  # Next position for a category appended at the end of the user's list.
  def self.next_position_for(user)
    where(user_id: user.id).maximum(:position).to_i + 1
  end
end
