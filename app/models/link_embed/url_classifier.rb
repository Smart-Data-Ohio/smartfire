class LinkEmbed::UrlClassifier
  # Up to this many generic (non-special) URLs per message render embeds.
  MAX_PER_MESSAGE = 3

  # A permissive absolute-URL scan; trailing punctuation is stripped after
  # the match, and every candidate still has to normalize and pass the
  # special-URL filters below.
  URL_PATTERN = %r{https?://[^\s<>"')\]]+}i

  # Angle-bracketed URLs are embed-suppressed, as in Discord: <https://…>
  # renders as a plain link with no card. The suppression set is built
  # from the raw Markdown source, where the brackets are still visible.
  SUPPRESSED_URL_PATTERN = %r{<(?<url>https?://[^<>\s]+)>}i

  # Internal permalinks (/rooms/1, /rooms/1/@2, /rooms/1/events/3 …) have
  # their own cards or need no card; they never get a generic embed.
  INTERNAL_PATH_PATTERN = %r{/rooms/\d+}i

  class << self
    # Unique normalized URLs worth embedding from the given rendered-text,
    # in order of appearance, capped at MAX_PER_MESSAGE. Special URLs
    # (GitHub PR, X post, LinkedIn post, Drive file, Fizzy, internal
    # permalink) are skipped: they have their own cards or need none.
    # Pass the normalized URLs wrapped in angle brackets in the Markdown
    # source as suppressed to leave them out as well.
    def extract(text, suppressed: [])
      return [] if text.blank?

      suppressed_keys = Set.new(Array(suppressed).compact)
      urls = []

      text.to_s.scan(URL_PATTERN) do
        candidate = clean_candidate(Regexp.last_match[0])
        normalized = LinkEmbed.normalize_url(candidate)
        next if normalized.nil? || urls.include?(normalized) || suppressed_keys.include?(normalized)
        next if special_url?(candidate)

        urls << normalized
        break if urls.size >= MAX_PER_MESSAGE
      end
      urls
    end

    # Normalized URLs wrapped in <…> in the raw Markdown source (and an
    # optional forward note), whose embeds are suppressed.
    def suppressed_urls(*sources)
      sources.compact_blank.flat_map do |source|
        source.to_s.scan(SUPPRESSED_URL_PATTERN).flatten.filter_map do |url|
          LinkEmbed.normalize_url(clean_candidate(url))
        end
      end.uniq
    end

    def special_url?(url)
      Github::PullRequestUrl.pull_request_url?(url) ||
        Twitter::PostUrl.post_url?(url) ||
        Linkedin::PostUrl.post_url?(url) ||
        Google::DriveLink.file_id(url).present? ||
        fizzy_url?(url) ||
        internal_url?(url)
    end

    # Fizzy workspace URLs have their own cards from the Fizzy
    # integration, so generic embeds leave them alone. Host-based on
    # purpose: any Fizzy host, present or future, is skipped.
    def fizzy_url?(url)
      host = host_of(url)
      host.present? && host.downcase.include?("fizzy")
    end

    def internal_url?(url)
      url.to_s.match?(INTERNAL_PATH_PATTERN)
    end

    # A URL at the end of a sentence picks up the period. Strip the
    # trailing characters that never end a URL rather than failing to
    # match the page.
    def clean_candidate(url)
      url.to_s.sub(/[.,;:!?}]+\z/, "")
    end

    private
      def host_of(url)
        URI.parse(url.to_s).host
      rescue URI::InvalidURIError
        nil
      end
  end
end
