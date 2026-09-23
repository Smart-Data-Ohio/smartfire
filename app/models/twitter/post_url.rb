module Twitter
  # Matches X/Twitter post URLs in message text. Accepts the canonical
  # /<handle>/status/<id> form (and the rare /statuses/ plural) plus the
  # handle-less /i/status/<id> and /i/web/status/<id> forms, on x.com and
  # twitter.com with optional www./mobile. subdomains, any scheme, and any
  # trailing path, query, or fragment (e.g. /photo/1, ?s=20).
  module PostUrl
    PATTERN = %r{
      https?://(?:www\.|mobile\.)?(?:twitter\.com|x\.com)/
      (?:
        i/(?:web/)?status/
        |
        (?<handle>[A-Za-z0-9_]{1,15})/status(?:es)?/
      )
      (?<id>\d{1,25})\b
    }x

    MAX_PER_MESSAGE = 4

    # Block-level tags whose boundaries separate words when flattening
    # HTML to text. fragment.text joins across them, which would glue a
    # URL onto adjacent prose (.../status/20thanks) and hide it from
    # extraction. code and pre are absent: they are removed beforehand.
    BLOCK_TAGS = %w[
      address article aside blockquote dd dialog div dl dt fieldset
      figcaption figure footer form h1 h2 h3 h4 h5 h6 header hgroup hr
      li main nav ol p section table td th tr ul
    ].freeze

    Reference = Data.define(:handle, :post_id)

    class << self
      # Unique post ids referenced by the given text, in order of appearance,
      # capped at MAX_PER_MESSAGE. Handles are kept for the API request but
      # dedupe is by id: the same post linked twice is one card.
      def extract(text)
        return [] if text.blank?

        references = []
        text.to_s.scan(PATTERN) do
          match = Regexp.last_match
          post_id = match[:id]
          next if references.any? { |reference| reference.post_id == post_id }

          references << Reference.new(match[:handle], post_id)
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
    end
  end
end
