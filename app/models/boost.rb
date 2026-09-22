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

  # Regional-indicator flag pair, e.g. 🇺🇸.
  FLAG_SEQUENCE_PATTERN = /\A\p{Regional_Indicator}{2}\z/
  # Keycap: a digit, # or * plus optional VS16 and U+20E3, e.g. 1️⃣ or #️⃣.
  KEYCAP_SEQUENCE_PATTERN = /\A[0-9#*]\uFE0F?\u20E3\z/

  def self.single_emoji?(content)
    content = content.to_s
    return false unless content.grapheme_clusters.one?

    # Keycaps and flags fall outside Emoji_Presentation and
    # Extended_Pictographic on some Unicode versions, so match them
    # explicitly as whole clusters.
    return true if content.match?(FLAG_SEQUENCE_PATTERN)
    return true if content.match?(KEYCAP_SEQUENCE_PATTERN)
    return true if content.match?(/[\p{Emoji_Presentation}\p{Extended_Pictographic}]/)
    return false unless content.match?(/\p{Emoji}/)

    # Fallback for text-default emoji on narrower Unicode versions: ZWJ
    # sequences, VS16 forms (e.g. ❤️), and skin-tone modifiers. Plain
    # digits and letters fail here: they carry no ZWJ, VS16 or modifier.
    if content.include?("\u200D")
      true
    elsif content.include?("\uFE0F")
      base = content.delete("\uFE0F")
      base.match?(/\p{Emoji}/) && !base.match?(/\A[0-9#*]\z/)
    elsif content.match?(/\p{Emoji_Modifier}/)
      content.gsub(/\p{Emoji_Modifier}/, "").match?(/\p{Emoji}/)
    else
      false
    end
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
