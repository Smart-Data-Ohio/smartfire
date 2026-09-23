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

      raw_by_normalized = raw_urls_by_normalized(text, suppressed)
      embeds_by_normalized = raw_by_normalized.keys.index_with do |normalized|
        LinkEmbed.for_reference(normalized)
      end

      message.link_embed_references.where.not(link_embed_id: embeds_by_normalized.values.map(&:id)).delete_all

      raw_by_normalized.each_with_index do |(normalized, raw_url), index|
        embed = embeds_by_normalized.fetch(normalized)
        reference = message.link_embed_references.find_or_create_by!(link_embed: embed)
        if reference.position != index || reference.url != raw_url
          reference.update!(position: index, url: raw_url)
        end

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

      # Each normalized key paired with the link as this message's
      # author wrote it, for the reference's own href. The raw URL stays
      # on the reference: the embed row is shared across rooms, so
      # storing it there would leak one room's fragments into another's
      # cards. LinkedIn URLs sync first, as before.
      def raw_urls_by_normalized(text, suppressed)
        raw_by_normalized = {}

        Linkedin::PostUrl.extract(text).each do |reference|
          normalized = LinkEmbed.normalize_url(reference.url)
          next if normalized.nil? || suppressed.include?(normalized)

          raw_by_normalized[normalized] ||= first_seen_url(text, normalized)
        end

        LinkEmbed::UrlClassifier.extract(text, suppressed: suppressed).each do |normalized|
          raw_by_normalized[normalized] ||= first_seen_url(text, normalized)
        end

        raw_by_normalized
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
