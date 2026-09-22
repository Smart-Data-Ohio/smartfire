module BoostsHelper
  # Accessible title for a counted reaction chip: the quick-reaction label,
  # the icon title, or the emoji name.
  def reaction_title(content)
    if (icon = shortcode_icon(content))
      icon.title
    else
      EmojiHelper::REACTIONS[content] || Emoji.find_by_unicode(content)&.name || content
    end
  end

  # Chip body for aggregated content: the icon image for brand and custom
  # shortcodes, the character for emoji (including literally stored emoji
  # shortcodes from before shortcode resolution).
  def reaction_chip_body(content)
    icon = shortcode_icon(content)

    if icon && !icon.emoji? && (url = Icons.image_url_for(icon))
      image_tag(url, class: icon_css_class(icon), alt: "", draggable: "false")
    elsif icon&.emoji?
      icon.character
    else
      h(content)
    end
  end

  # Brand and workspace icon shortcode boosts render the icon; everything
  # else renders as plain text exactly like before.
  def boost_content_html(boost)
    if boost.shortcode_content?
      icon = Icons.find(boost.content.to_s[1...-1])

      # Resolved through the registry so an icon whose asset is missing
      # falls back to its literal text instead of raising at render time.
      if icon && (url = Icons.image_url_for(icon))
        return image_tag(url,
          class: icon_css_class(icon), alt: ":#{icon.name}:", title: icon.title, draggable: "false")
      end
    end

    h(boost.content)
  end

  private
    def shortcode_icon(content)
      content = content.to_s
      Icons.find(content[1...-1]) if content.match?(Boost::SHORTCODE_CONTENT_PATTERN)
    end

    # Icons.image_url_for only resolves brands and workspace icons, so
    # anything reaching here is one of the two.
    def icon_css_class(icon)
      icon.brand? ? "icon icon--brand" : "icon icon--custom"
    end
end
