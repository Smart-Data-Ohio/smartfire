class Boost < ApplicationRecord
  belongs_to :message, touch: true
  belongs_to :booster, class_name: "User", default: -> { Current.user }

  scope :ordered, -> { order(:created_at) }

  before_validation :resolve_shortcode_content

  SHORTCODE_CONTENT_PATTERN = /\A:[a-z0-9_]+:\z/

  # Emoji shortcodes resolve to the character itself, so they render and
  # count exactly like an emoji typed directly. Brand and workspace icon
  # shortcodes stay as :name:, canonicalised so aliases share one reaction
  # chip, and render through BoostsHelper. Unknown shortcodes stay literal
  # text, as before.
  def self.resolve_content(content)
    content = content.to_s.strip
    return content unless content.match?(SHORTCODE_CONTENT_PATTERN)

    case (icon = Icons.find(content[1...-1]))
    when Icons::Emoji then icon.character
    when Icons::Brand, Icons::Custom then ":#{icon.name}:"
    else content
    end
  end

  # Content that is exactly a :shortcode: (for example from icon autocomplete).
  def shortcode_content?
    content.to_s.match?(SHORTCODE_CONTENT_PATTERN)
  end

  # Content that aggregates into one counted chip with a toggle: a single
  # emoji, or a :shortcode: for a known emoji or icon. Free text and
  # unknown shortcodes stay per-person legacy chips.
  def self.reaction?(content)
    content = content.to_s.strip
    single_emoji?(content) || known_shortcode?(content)
  end

  def self.single_emoji?(content)
    content.grapheme_clusters.one? &&
      content.match?(/[\p{Emoji_Presentation}\p{Extended_Pictographic}]/)
  end

  def self.known_shortcode?(content)
    content.match?(SHORTCODE_CONTENT_PATTERN) && Icons.find(content[1...-1]).present?
  end

  private
    def resolve_shortcode_content
      # resolve_content strips, so ":name: " from icon autocomplete resolves
      # instead of storing as literal text.
      self.content = self.class.resolve_content(content)
    end
end
