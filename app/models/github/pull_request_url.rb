module Github
  # Matches GitHub pull request URLs in message text. Accepts both the
  # canonical /pull/<number> form and /pulls/<number> variants, with any
  # trailing path, query, or fragment (e.g. /files, ?diff=split).
  module PullRequestUrl
    # Owner and repo segments exclude exactly "." and "..", which GitHub
    # never issues as names, so traversal-looking URLs never match.
    PATTERN = %r{
      https://github\.com/
      (?<owner>(?!\.\.?/)[A-Za-z0-9_.-]+)/
      (?<repo>(?!\.\.?/)[A-Za-z0-9_.-]+)/
      (?:pull|pulls)/
      (?<number>\d+)\b
    }x

    MAX_PER_MESSAGE = 4

    # Block-level tags whose boundaries separate words when flattening
    # HTML to text. fragment.text joins across them, which would glue a
    # URL onto adjacent prose (.../pull/12thanks) and hide it from
    # extraction. code and pre are absent: they are removed beforehand.
    BLOCK_TAGS = %w[
      address article aside blockquote dd dialog div dl dt fieldset
      figcaption figure footer form h1 h2 h3 h4 h5 h6 header hgroup hr
      li main nav ol p section table td th tr ul
    ].freeze

    Reference = Data.define(:owner, :repo, :number)

    class << self
      # Unique (owner, repo, number) triples referenced by the given text,
      # in order of appearance, capped at MAX_PER_MESSAGE.
      def extract(text)
        return [] if text.blank?

        references = []
        text.to_s.scan(PATTERN) do
          match = Regexp.last_match
          reference = Reference.new(match[:owner], match[:repo], match[:number].to_i)
          next if references.include?(reference)

          references << reference
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

      def pull_request_url?(url)
        url.to_s.match?(PATTERN)
      end
    end
  end
end
