module Fizzy
  # Matches Fizzy card URLs in message text. The account segment is the
  # numeric account id Fizzy puts in card URLs
  # (https://app.fizzy.do/<account_id>/cards/<number>); letters are
  # accepted too so custom slugs keep matching. Any trailing path, query,
  # or fragment matches (e.g. a comment anchor), so comment links unfurl
  # their card like GitHub /files links do.
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
        text.to_s.scan(PATTERN) do
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
        url.to_s.match?(PATTERN)
      end
    end
  end
end
