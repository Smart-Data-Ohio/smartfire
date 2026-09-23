module LinkEmbedsHelper
  # Generic embed references on a message, in link order. Sorting in Ruby
  # rather than with an `order` scope, because applying a scope to an
  # association builds a fresh relation and so ignores already loaded rows —
  # one extra query per message rendered. (Same reason `ordered_boosts`
  # exists.) Rendering a stale card re-enqueues its fetch, so an expired
  # result refreshes on view.
  #
  # References, not embeds: each card links to its own message's raw URL
  # (reference.display_url), never to anything on the room-shared row.
  def link_embed_cards_for(message)
    return [] if message.embeds_suppressed?

    references = ordered_link_embed_references(message).reject { |reference| reference.link_embed.linkedin? }
    references.each { |reference| request_link_embed_fetch(reference.link_embed) }
    # Unusable fetches render nothing (the author's link is the fallback),
    # so they are left out: an iteration that rendered an empty string
    # would leave whitespace text nodes behind and defeat the container's
    # :empty rule.
    references.select { |reference| reference.link_embed.usable? }
  end

  private
    def ordered_link_embed_references(message)
      message.link_embed_references
        .sort_by { |reference| [ reference.position, reference.id ] }
    end

    # Enqueue a fetch for a missing or expired card, bounded two ways: once
    # per embed per render (this helper instance lives for one request, so
    # the set needs no clearing), and at most one enqueue per embed per
    # window across renders via the record's fetch-request claim.
    def request_link_embed_fetch(embed)
      return unless embed.needs_fetch?
      return unless (@link_embed_fetches ||= Set.new).add?(embed.id)

      LinkEmbed::FetchJob.perform_later(embed) if embed.claim_fetch_request!
    end
end
