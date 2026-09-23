class LinkEmbed < ApplicationRecord
  SUCCESS_TTL = 24.hours
  NEGATIVE_TTL = 1.hour
  FETCH_REQUEST_WINDOW = 10.minutes

  MAX_TITLE_CHARS = 300
  MAX_DESCRIPTION_CHARS = 1000
  MAX_SITE_NAME_CHARS = 100

  has_many :link_embed_references, dependent: :destroy, inverse_of: :link_embed
  has_many :messages, through: :link_embed_references

  validates :normalized_url, presence: true, uniqueness: true
  validates :title, length: { maximum: MAX_TITLE_CHARS }, allow_nil: true
  validates :description, length: { maximum: MAX_DESCRIPTION_CHARS }, allow_nil: true
  validates :site_name, length: { maximum: MAX_SITE_NAME_CHARS }, allow_nil: true

  after_update_commit :broadcast_card_updates

  # Canonical cache key for a URL: downcased host, default ports dropped,
  # fragment dropped, a trailing slash beyond the root dropped. The query
  # string is kept: it usually selects the page. Returns nil for garbage.
  def self.normalize_url(url)
    parsed = URI.parse(url.to_s.strip)
    return unless parsed.is_a?(URI::HTTP) && parsed.host.present?

    key = +"#{parsed.scheme.downcase}://#{parsed.host.downcase}"
    key << ":#{parsed.port}" if parsed.port && parsed.port != parsed.default_port
    path = parsed.path.presence || "/"
    path = path.chomp("/") if path != "/" && path.end_with?("/")
    key << path
    key << "?#{parsed.query}" if parsed.query.present?
    key
  rescue URI::InvalidURIError
    nil
  end

  # Find or create the cached row for a referenced URL. Safe to call
  # concurrently: a lost insert race falls back to finding the winner.
  def self.for_reference(normalized_url, url: nil)
    create_with(url: url).find_or_create_by!(normalized_url: normalized_url)
  rescue ActiveRecord::RecordNotUnique
    find_by!(normalized_url: normalized_url)
  end

  # True when no fetch has completed yet or the cached result (positive
  # or negative) has outlived its TTL.
  def needs_fetch?
    expires_at.nil? || expires_at <= Time.current
  end

  def fresh?
    !needs_fetch?
  end

  def fetch_requested_recently?
    fetch_requested_at.present? && fetch_requested_at >= FETCH_REQUEST_WINDOW.ago
  end

  # Atomically claim the right to enqueue a fetch for this URL: at most one
  # caller per URL wins per window, however many messages race. The update
  # skips callbacks, so claiming never broadcasts a card update.
  def claim_fetch_request!
    return false if fetch_requested_recently?

    claimed = self.class.where(id: id)
      .where("fetch_requested_at IS NULL OR fetch_requested_at < ?", FETCH_REQUEST_WINDOW.ago)
      .update_all(fetch_requested_at: Time.current) == 1
    self.fetch_requested_at = Time.current if claimed
    claimed
  end

  # Whether the fetch produced something worth rendering. A row with
  # neither title nor description renders nothing (generic) or a compact
  # link chip (LinkedIn, whose pages are usually login-gated).
  def usable?
    title.present? || description.present?
  end

  def linkedin?
    Linkedin::PostUrl.post_url?(url.presence || normalized_url)
  end

  # The first-seen URL for display, falling back to the cache key.
  def display_url
    url.presence || normalized_url
  end

  def broadcast_card_updates
    target, partial = linkedin? ? [ :linkedin_cards, "linkedin/posts/cards" ] : [ :link_embed_cards, "link_embeds/cards" ]

    referencing_messages.find_each do |message|
      Turbo::StreamsChannel.broadcast_replace_to(
        message.message_stream_target, :messages,
        target: ActionView::RecordIdentifier.dom_id(message, target),
        partial: partial,
        locals: { message: message },
        attributes: { maintain_scroll: true }
      )
    end
  end

  private
    def referencing_messages
      Message.where(id: link_embed_references.select(:message_id))
    end
end
