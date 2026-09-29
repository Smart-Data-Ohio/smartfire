require "strscan"

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
  #
  # Each pass (URLs, then email addresses) finds the values in its text once,
  # and each match looks its position up in them, so a body with many matches
  # stays linear rather than rescanning everything before every match.
  module AutoLinkOutsideAttributeValues
    # The rails_autolink 1.1.8 private methods this wraps. A gem upgrade that
    # renames or reshapes them would silently bypass the protection, so boot
    # fails instead.
    WRAPPED_METHODS = {
      auto_link_urls: [ [ :req, :text ], [ :opt, :html_options ], [ :opt, :options ] ],
      auto_link_email_addresses: [ [ :req, :text ], [ :opt, :html_options ], [ :opt, :options ] ],
      auto_linked?: [ [ :req, :left ], [ :req, :right ] ]
    }

    def self.prepend_to(helper)
      return if helper.ancestors.include?(self)

      WRAPPED_METHODS.each do |name, parameters|
        unless helper.private_method_defined?(name, false) && helper.instance_method(name).parameters == parameters
          raise "rails_autolink's #{name} changed; review #{__FILE__} before upgrading"
        end
      end

      helper.prepend self
    end

    private
      def auto_link_urls(text, *)
        with_attribute_values_in(text) { super }
      end

      def auto_link_email_addresses(text, *)
        with_attribute_values_in(text) { super }
      end

      # left is everything before the match in the text being linked, so its
      # byte size is the match's offset there.
      def auto_linked?(left, right)
        inside_attribute_value?(left.bytesize) || super
      end

      def with_attribute_values_in(text)
        @auto_link_attribute_values = quoted_attribute_values(text)
        yield
      ensure
        @auto_link_attribute_values = nil
      end

      # Byte ranges of the quoted attribute values, ascending. A value without
      # a closing quote isn't one.
      def quoted_attribute_values(html)
        scanner = StringScanner.new(html)
        values = []
        in_tag = false

        while scanner.skip_until(/[<>"]/)
          case scanner.matched
          when "<" then in_tag = true
          when ">" then in_tag = false
          else
            if in_tag
              start = scanner.pos
              break unless scanner.skip_until(/"/)
              values << (start...scanner.pos - 1)
            end
          end
        end

        values
      end

      def inside_attribute_value?(offset)
        value = @auto_link_attribute_values&.bsearch { |range| range.end > offset }
        !value.nil? && value.begin <= offset
      end
  end
end

ActiveSupport.on_load(:action_view) do
  RailsExt::AutoLinkOutsideAttributeValues.prepend_to ActionView::Helpers::TextHelper
end
