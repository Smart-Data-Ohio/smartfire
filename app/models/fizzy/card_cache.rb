class Fizzy::CardCache < ApplicationRecord
  self.table_name = "fizzy_card_caches"

  STALE_AFTER = 5.minutes

  # fetch_error value when Fizzy answers 404 or 403: the viewer cannot
  # access the card, so the frame renders the minimal link chip instead
  # of an error.
  NOT_FOUND = "not_found"

  belongs_to :card, class_name: "Fizzy::Card", foreign_key: :fizzy_card_id
  belongs_to :user

  validates :fizzy_card_id, uniqueness: { scope: :user_id }

  # Find or create the cache row for a viewer and card. Safe to call
  # concurrently: a lost insert race falls back to the winner's row.
  def self.for_viewer(card:, user:)
    find_or_create_by!(fizzy_card_id: card.id, user_id: user.id)
  rescue ActiveRecord::RecordNotUnique
    find_by!(fizzy_card_id: card.id, user_id: user.id)
  end

  def stale?
    fetched_at.nil? || fetched_at < STALE_AFTER.ago
  end

  def not_found?
    fetch_error == NOT_FOUND
  end

  def fetch_requested_recently?
    fetch_requested_at.present? && fetch_requested_at >= STALE_AFTER.ago
  end

  # Atomically claim the right to enqueue a fetch for this viewer and
  # card: at most one caller per row wins per staleness window, however
  # many renders race. The update skips callbacks, so claiming never
  # broadcasts a card update.
  def claim_fetch_request!
    return false if fetch_requested_recently?

    claimed = self.class.where(id: id)
      .where("fetch_requested_at IS NULL OR fetch_requested_at < ?", STALE_AFTER.ago)
      .update_all(fetch_requested_at: Time.current) == 1
    self.fetch_requested_at = Time.current if claimed
    claimed
  end

  # Give up a claim whose enqueue failed (the queue is down): the next
  # render may try again instead of waiting out the staleness window.
  # Skips callbacks, so releasing never broadcasts a card update.
  def release_fetch_request!
    self.class.where(id: id).update_all(fetch_requested_at: nil)
    self.fetch_requested_at = nil
  end
end
