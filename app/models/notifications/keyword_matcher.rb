module Notifications
  # Matches keyword alert phrases against one message body: every
  # distinct phrase across all candidate recipients compiles into its
  # own case-insensitive word-boundary pattern, checked independently.
  # A single alternation scanned once would let a longer phrase consume
  # a shorter overlapping one ("deploy failed" swallowing "deploy" for
  # another user), so each phrase gets its own match pass instead.
  module KeywordMatcher
    class << self
      # phrases_by_user_id maps user ids to phrase arrays. Returns the
      # ids of users with at least one match. Pure Ruby, no queries.
      def matching_user_ids(phrases_by_user_id, text)
        return [] if text.blank? || phrases_by_user_id.blank?

        users_by_phrase = {}
        phrases_by_user_id.each do |user_id, phrases|
          Array(phrases).each do |phrase|
            normalized = phrase.to_s.strip.downcase
            next if normalized.blank?

            (users_by_phrase[normalized] ||= []) << user_id
          end
        end
        return [] if users_by_phrase.empty?

        patterns = users_by_phrase.keys.index_with { |phrase| compile(phrase) }
        matched = Set.new
        users_by_phrase.each do |phrase, user_ids|
          matched.merge(user_ids) if patterns.fetch(phrase).match?(text)
        end
        matched.to_a
      end

      private
        # Lookarounds instead of \b so phrases ending in punctuation
        # ("v1.2 (rc)") still match at a space; for plain words the two
        # are equivalent.
        def compile(phrase)
          Regexp.new("(?<!\\w)#{Regexp.escape(phrase)}(?!\\w)", Regexp::IGNORECASE)
        end
    end
  end
end
