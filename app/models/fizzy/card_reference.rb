class Fizzy::CardReference < ApplicationRecord
  self.table_name = "fizzy_card_references"

  belongs_to :message
  belongs_to :card, class_name: "Fizzy::Card", foreign_key: :fizzy_card_id

  validates :fizzy_card_id, uniqueness: { scope: :message_id }
end
