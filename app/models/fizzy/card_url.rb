module Fizzy
  # Matches Fizzy card URLs in message text. The account segment is the
  # numeric account id Fizzy puts in card URLs
  # (https://app.fizzy.do/<account_id>/cards/<number>); letters are
  # accepted too so custom slugs keep matching. Any trailing path, query,
  # or fragment matches (e.g. a comment anchor), so comment links unfurl
  # their card like GitHub /files links do.
  #
  # The host comes from the configured Fizzy origin
  # (FIZZY_API_BASE_URL): only card URLs on that host are extracted, so
  # a self-hosted Fizzy works end to end.
  module CardUrl
    PATTERN = %r{
      https://app\.fizzy\.do/
      (?<account_id>[A-Za-z0-9_-]+)/
      cards/
      (?<number>\d+)\b
    }x

    MAX_PER_MESSAGE = 4

    Reference = Data.define(:account_id, :number)

    class << self
      # Unique (account_id, number) pairs referenced by the given text,
      # in order of appearance, capped at MAX_PER_MESSAGE.
      def extract(text)
        return [] if text.blank?

        references = []
        text.to_s.scan(pattern) do
          match = Regexp.last_match
          reference = Reference.new(match[:account_id], match[:number].to_i)
          next if references.include?(reference)

          references << reference
          break if references.size >= MAX_PER_MESSAGE
        end
        references
      end

      # The text of rendered message HTML outside code spans and fenced
      # blocks, for reference extraction that ignores URLs quoted in
      # code. Shared with the PR cards so both stay in sync.
      def non_code_text(html)
        Github::PullRequestUrl.non_code_text(html)
      end

      def card_url?(url)
        url.to_s.match?(pattern)
      end

      # Extraction pattern for the configured Fizzy host. An optional
      # port is accepted so hosts with a non-default port match; an
      # unparseable base URL falls back to the default pattern.
      def pattern
        uri = URI.parse(Client.api_base_url)
        host = uri.host.presence or raise URI::InvalidURIError
        scheme = uri.scheme == "http" ? "http" : "https"
        %r{
          #{scheme}://#{Regexp.escape(host)}(?::\d+)?/
          (?<account_id>[A-Za-z0-9_-]+)/
          cards/
          (?<number>\d+)\b
        }x
      rescue URI::InvalidURIError
        PATTERN
      end
    end
  end
end
