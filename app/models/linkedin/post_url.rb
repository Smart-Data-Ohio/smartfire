module Linkedin
  # Matches LinkedIn post URLs in message text. Accepts the /posts/<slug>
  # form and the /feed/update/<urn> form for activity, share, and ugcPost
  # URNs, on linkedin.com with an optional www. subdomain, any scheme, and
  # any trailing path, query, or fragment.
  module PostUrl
    PATTERN = %r{
      https?://(?:www\.)?linkedin\.com/
      (?:
        posts/(?<slug>[^/?#\s]+)
        |
        feed/update/(?<urn>urn:li:(?:activity|share|ugcPost):[^/?#\s]+)
      )
    }x

    MAX_PER_MESSAGE = 3

    # Block-level tags whose boundaries separate words when flattening
    # HTML to text. fragment.text joins across them, which would glue a
    # URL onto adjacent prose (.../posts/abc123thanks) and hide it from
    # extraction. code and pre are absent: they are removed beforehand.
    BLOCK_TAGS = %w[
      address article aside blockquote dd dialog div dl dt fieldset
      figcaption figure footer form h1 h2 h3 h4 h5 h6 header hgroup hr
      li main nav ol p section table td th tr ul
    ].freeze

    Reference = Data.define(:url, :urn)

    class << self
      # Unique post URLs referenced by the given text, in order of
      # appearance, capped at MAX_PER_MESSAGE. Dedupe is by the matched
      # URL without its query string or fragment: the same post linked
      # twice is one card.
      def extract(text)
        return [] if text.blank?

        references = []
        keys = Set.new
        text.to_s.scan(PATTERN) do
          match = Regexp.last_match
          key = dedupe_key(match[0])
          next if keys.include?(key)

          keys << key
          references << Reference.new(match[0], match[:urn])
          break if references.size >= MAX_PER_MESSAGE
        end
        references
      end

      # The text of rendered message HTML outside code spans and fenced
      # blocks, for reference extraction that ignores URLs quoted in
      # code. Hrefs outside code are kept alongside the visible text so
      # labeled links still resolve.
      def non_code_text(html)
        fragment = Nokogiri::HTML5.fragment(html.to_s)
        fragment.css("code, pre").remove
        fragment.css("br").each { |br| br.replace("\n") }
        fragment.css(BLOCK_TAGS.join(",")).each { |element| element.after("\n") }

        [ fragment.text, *fragment.css("a[href]").map { |link| link["href"] } ].join("\n")
      end

      def post_url?(url)
        url.to_s.match?(PATTERN)
      end

      # The official LinkedIn embed player URL for a feed/update reference.
      # /posts/ URLs carry no URN, so they render a fetched card (or a link
      # chip when login-gated) with no embed button.
      def embed_url_for(reference_or_url)
        urn = reference_or_url.is_a?(Reference) ? reference_or_url.urn : extract_urn(reference_or_url)
        "https://www.linkedin.com/embed/feed/update/#{urn}" if urn.present?
      end

      private
        def dedupe_key(matched_url)
          matched_url.to_s.split(/[?#]/, 2).first
        end

        def extract_urn(url)
          url.to_s.match(PATTERN)&.[](:urn)
        end
    end
  end
end
