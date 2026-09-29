module RailsExt
  # rails_autolink finds URLs and email addresses with regular expressions over
  # serialized HTML, and skips a match that auto_linked? places inside a tag: a
  # "<" with no ">" after it. Nokogiri leaves "<" and ">" raw inside quoted
  # attribute values, so in a stored value like title="x> http://…" the URL
  # reads as text and gets an anchor inserted into the value. The anchor's own
  # quotes end the attribute early, and the rest of the value, which
  # sanitization had passed as an inert string, is rendered as markup.
  #
  # A match that starts inside a quoted attribute value counts as linked, so the
  # value is left as it is. Nokogiri double-quotes every attribute value and
  # escapes "<" and ">" in text, so the double quotes between a "<" and its ">"
  # delimit every attribute value.
  module AutoLinkOutsideAttributeValues
    private
      def auto_linked?(left, right)
        inside_attribute_value?(left, right) || super
      end

      # The value's closing quote comes after the match, which can't contain one.
      def inside_attribute_value?(left, right)
        in_tag = in_value = false

        left.scan(/[<>"]/) do |char|
          case char
          when "<" then in_tag = true unless in_value
          when ">" then in_tag = false unless in_value
          else in_value = !in_value if in_tag
          end
        end

        in_value && right.include?('"')
      end
  end
end

ActiveSupport.on_load(:action_view) do
  ActionView::Helpers::TextHelper.prepend RailsExt::AutoLinkOutsideAttributeValues
end
