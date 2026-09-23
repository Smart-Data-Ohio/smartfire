class LinkEmbedReference < ApplicationRecord
  belongs_to :message
  belongs_to :link_embed

  validates :link_embed_id, uniqueness: { scope: :message_id }

  # The link as this message's author wrote it, for card hrefs. Card
  # partials must use this, never anything on the shared embed row: the
  # row is shared across rooms, and rendering its URL would leak the
  # first poster's fragments into other rooms' cards. Falls back to the
  # normalized key when the raw link was never stored.
  def display_url
    url.presence || link_embed.normalized_url
  end
end
