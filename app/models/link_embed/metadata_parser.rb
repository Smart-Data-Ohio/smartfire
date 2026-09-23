class LinkEmbed::MetadataParser
  # OpenGraph first, then the Twitter-card equivalents, then the plain
  # document fallbacks. og:image:secure_url wins over og:image when both
  # are present: some pages serve the plain one over http.
  TITLE_SELECTORS = [
    [ :property, "og:title" ], [ :name, "og:title" ],
    [ :property, "twitter:title" ], [ :name, "twitter:title" ]
  ].freeze
  DESCRIPTION_SELECTORS = [
    [ :property, "og:description" ], [ :name, "og:description" ],
    [ :property, "twitter:description" ], [ :name, "twitter:description" ],
    [ :name, "description" ]
  ].freeze
  SITE_NAME_SELECTORS = [
    [ :property, "og:site_name" ], [ :name, "og:site_name" ],
    [ :property, "twitter:site" ], [ :name, "twitter:site" ]
  ].freeze
  IMAGE_SELECTORS = [
    [ :property, "og:image:secure_url" ], [ :name, "og:image:secure_url" ],
    [ :property, "og:image" ], [ :name, "og:image" ],
    [ :property, "twitter:image:src" ], [ :name, "twitter:image:src" ],
    [ :property, "twitter:image" ], [ :name, "twitter:image" ]
  ].freeze

  Result = Data.define(:title, :description, :site_name, :image_url)

  class << self
    # Everything the card renders, from a fetched HTML document. All
    # strings are untrusted page content: tags stripped, entities
    # decoded by the parser, truncated to the embed column limits.
    def parse(html, base_url:)
      document = Nokogiri::HTML(html.to_s)

      Result.new(
        title: clean(first_content(document, TITLE_SELECTORS) || document.at_css("title")&.text,
          limit: LinkEmbed::MAX_TITLE_CHARS),
        description: clean(first_content(document, DESCRIPTION_SELECTORS),
          limit: LinkEmbed::MAX_DESCRIPTION_CHARS),
        site_name: clean(first_content(document, SITE_NAME_SELECTORS) || host_of(base_url),
          limit: LinkEmbed::MAX_SITE_NAME_CHARS),
        image_url: absolute_url(first_content(document, IMAGE_SELECTORS), base_url)
      )
    end

    private
      def first_content(document, selectors)
        selectors.each do |attribute, name|
          tag = document.at_xpath("//meta[translate(@#{attribute}, 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz')='#{name}']")
          content = tag&.[]("content").to_s.strip
          return content if content.present?
        end
        nil
      end

      # Plain text for the model: tags stripped, then entities decoded.
      # full_sanitizer returns entity-escaped text, and without the decode
      # ERB would escape it a second time on render ("Tom &amp; Jerry").
      # Decoding is safe here because the tags are already gone and the
      # card escapes the value again on render.
      def clean(value, limit:)
        stripped = ActionView::Base.full_sanitizer.sanitize(value.to_s).strip
        CGI.unescapeHTML(stripped).presence&.truncate(limit, omission: "")
      end

      # og:image is supposed to be absolute, but pages in the wild serve
      # root-relative and relative paths; resolve them against the page.
      # Only http(s) targets survive: the card renders the image directly.
      def absolute_url(value, base_url)
        return if value.blank?

        resolved = URI.join(base_url.to_s, value.to_s.strip).to_s
        parsed = URI.parse(resolved)
        resolved if parsed.is_a?(URI::HTTP) && parsed.host.present?
      rescue URI::InvalidURIError
        nil
      end

      def host_of(url)
        URI.parse(url.to_s).host
      rescue URI::InvalidURIError
        nil
      end
  end
end
