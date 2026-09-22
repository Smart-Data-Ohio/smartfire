class Search < ApplicationRecord
  belongs_to :user

  before_create :set_dedup_key
  after_create :trim_recent_searches

  scope :ordered, -> { order(updated_at: :desc) }

  class << self
    # Find-first for the common repeat (and for legacy rows without a dedup
    # key), with a race-safe create behind it: concurrent first records
    # collide on the unique index and the loser finds the winner's row.
    def record(query)
      find_or_create_by(query: query).touch
    end
  end

  private
    # New rows are deduplicated by the unique index on [user_id, dedup_key];
    # legacy rows keep a NULL key so they never collide with it.
    def set_dedup_key
      self.dedup_key ||= query
    end

    def trim_recent_searches
      user.searches.excluding(user.searches.ordered.limit(10)).destroy_all
    end
end
