class LinkEmbed::ReferenceSync
  class << self
    # Reconciles a message's link-embed references with the URLs its
    # content currently contains, and enqueues a fetch for every
    # referenced URL whose cached result is missing or expired.
    # Idempotent: re-running with unchanged content enqueues nothing new.
    #
    # Only Markdown messages sync: legacy Trix bodies already carry their
    # stored OpenGraph attachments, and a second card beneath would render
    # every old unfurl twice. LinkedIn post URLs sync through the same
    # rows; the card partials split them back out by URL.
    #
    # Pass `enqueue_fetches: false` from contexts that cannot reach Redis,
    # such as migrations: references are still created, and each card
    # enqueues its own fetch the first time it renders.
    def call(message, enqueue_fetches: true)
      unless message.markdown? || message.forwarded_markdown?
        message.link_embed_references.delete_all
        return
      end

      suppressed = LinkEmbed::UrlClassifier.suppressed_urls(message.markdown_source, message.forward_note)
      text = reference_text(message)

      linkedin_urls = Linkedin::PostUrl.extract(text)
        .map { |reference| LinkEmbed.normalize_url(reference.url) }
        .compact
        .reject { |normalized| suppressed.include?(normalized) }

      generic_urls = LinkEmbed::UrlClassifier.extract(text, suppressed: suppressed)

      normalized_urls = (linkedin_urls + generic_urls).uniq

      embeds = normalized_urls.map do |normalized|
        LinkEmbed.for_reference(normalized, url: first_seen_url(text, normalized))
      end

      message.link_embed_references.where.not(link_embed_id: embeds.map(&:id)).delete_all

      embeds.each_with_index do |embed, index|
        reference = message.link_embed_references.find_or_create_by!(link_embed: embed)
        reference.update!(position: index) if reference.position != index

        if enqueue_fetches && !message.embeds_suppressed? && embed.needs_fetch?
          LinkEmbed::FetchJob.perform_later(embed) if embed.claim_fetch_request!
        end
      end
    rescue ActiveRecord::RecordNotUnique
      retry
    end

    private
      def reference_text(message)
        [ Linkedin::PostUrl.non_code_text(message.body.body&.to_html), message.forward_note ].compact_blank.join("\n")
      end

      # The link as the author wrote it, for display. The classifier only
      # returns normalized keys, so re-scan the text for the first raw
      # candidate that normalizes to the key.
      def first_seen_url(text, normalized)
        text.to_s.scan(LinkEmbed::UrlClassifier::URL_PATTERN) do
          candidate = LinkEmbed::UrlClassifier.clean_candidate(Regexp.last_match[0])
          return candidate if LinkEmbed.normalize_url(candidate) == normalized
        end
        normalized
      end
  end
end
