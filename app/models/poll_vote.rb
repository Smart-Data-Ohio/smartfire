class PollVote < ApplicationRecord
  belongs_to :poll
  belongs_to :poll_option
  belongs_to :user

  validates :user_id, uniqueness: { scope: :poll_option_id }
  validate :option_must_belong_to_poll

  private
    def option_must_belong_to_poll
      return if poll_option.nil? || poll.nil?
      return if poll_option.poll_id == poll.id

      errors.add :poll_option, "is not part of this poll"
    end
end
