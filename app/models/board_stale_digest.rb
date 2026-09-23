# The idempotency claim behind one board's daily stale-work digest. The
# dispatcher inserts this row (unique per board and day) before posting,
# so a digest goes out at most once a day even across runner restarts.
# The posted quiet system note is linked back for the board page.
class BoardStaleDigest < ApplicationRecord
  belongs_to :room
  belongs_to :message, optional: true

  validates :digest_on, presence: true, uniqueness: { scope: :room_id }
end
