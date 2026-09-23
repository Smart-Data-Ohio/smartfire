class LinkEmbedReference < ApplicationRecord
  belongs_to :message
  belongs_to :link_embed

  validates :link_embed_id, uniqueness: { scope: :message_id }
end
