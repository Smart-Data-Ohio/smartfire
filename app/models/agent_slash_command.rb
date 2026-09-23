class AgentSlashCommand < ApplicationRecord
  NAME_FORMAT = /\A[a-z][a-z0-9_-]{0,31}\z/
  DESCRIPTION_LIMIT = 140

  belongs_to :agent
  belongs_to :room

  normalizes :name, with: ->(name) { name.to_s.strip.downcase.presence }
  normalizes :description, with: ->(description) { description.to_s.strip.presence }

  validates :name, presence: true, format: { with: NAME_FORMAT }, uniqueness: { scope: :room_id }
  validates :description, length: { maximum: DESCRIPTION_LIMIT }, allow_nil: true
  validate :name_must_not_shadow_builtin

  scope :ordered, -> { order(:name, :id) }

  private
    def name_must_not_shadow_builtin
      return if name.blank?
      return unless SlashCommands::Registry.builtin?(name)

      errors.add :name, "is already a built-in command"
    end
end
