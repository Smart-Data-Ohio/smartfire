module Notifications
  # Matches keyword alert phrases against one message body in a single
  # pass: every distinct phrase across all candidate recipients compiles
  # into one case-insensitive word-boundary pattern, scanned once.
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

        pattern = compile(users_by_phrase.keys)
        matched = Set.new
        text.scan(pattern) { |match| users_by_phrase[match.downcase]&.each { |id| matched << id } }
        matched.to_a
      end

      private
        # Lookarounds instead of \b so phrases ending in punctuation
        # ("v1.2 (rc)") still match at a space; for plain words the two
        # are equivalent.
        def compile(phrases)
          alternation = phrases.sort_by(&:length).reverse.map { |phrase| Regexp.escape(phrase) }.join("|")
          Regexp.new("(?<!\\w)(?:#{alternation})(?!\\w)", Regexp::IGNORECASE)
        end
    end
  end
end
