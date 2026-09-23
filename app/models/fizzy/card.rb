class Fizzy::Card < ApplicationRecord
  self.table_name = "fizzy_cards"

  has_many :card_references, class_name: "Fizzy::CardReference",
    foreign_key: :fizzy_card_id, dependent: :destroy, inverse_of: :card
  has_many :messages, through: :card_references
  has_many :card_caches, class_name: "Fizzy::CardCache",
    foreign_key: :fizzy_card_id, dependent: :destroy, inverse_of: :card

  validates :account_id, presence: true
  validates :number, presence: true, numericality: { only_integer: true, greater_than: 0 }
  validates :number, uniqueness: { scope: :account_id }

  def web_url
    "#{Fizzy::Client.api_base_url}/#{account_id}/cards/#{number}"
  end

  # Find or create the record for a referenced card. Safe to call
  # concurrently: a lost insert race falls back to finding the winner's row.
  def self.for_reference(account_id:, number:)
    create_with({}).find_or_create_by!(account_id: account_id.to_s, number: number)
  rescue ActiveRecord::RecordNotUnique
    find_by!(account_id: account_id.to_s, number: number)
  end

  # Refresh every referencing message's card container after one viewer's
  # cache changed. The container holds only lazy turbo-frames with src, so
  # the broadcast itself carries no card content: each viewer's frames
  # reload through the per-viewer card endpoint with their own token.
  def broadcast_card_updates
    referencing_messages.find_each do |message|
      Turbo::StreamsChannel.broadcast_replace_to(
        message.message_stream_target, :messages,
        target: ActionView::RecordIdentifier.dom_id(message, :fizzy_cards),
        partial: "fizzy/cards/cards",
        locals: { message: message },
        attributes: { maintain_scroll: true }
      )
    end
  end

  private
    def referencing_messages
      Message.where(id: card_references.select(:message_id))
    end
end
