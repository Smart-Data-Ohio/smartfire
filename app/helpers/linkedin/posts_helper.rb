module Linkedin
  module PostsHelper
    # LinkedIn post references on a message, in link order. Sorted in
    # Ruby off the preloaded references (see LinkEmbedsHelper) so rendering
    # a page of messages costs no per-message queries. Rendering a stale
    # card re-enqueues its fetch, so an expired result refreshes on view.
    #
    # References, not embeds: each card links to its own message's raw URL
    # (reference.display_url), never to anything on the room-shared row.
    def linkedin_post_cards_for(message)
      return [] if message.embeds_suppressed?

      references = message.link_embed_references
        .sort_by { |reference| [ reference.position, reference.id ] }
        .select { |reference| reference.link_embed.linkedin? }
      references.each { |reference| request_linkedin_post_fetch(reference.link_embed) }
      references
    end

    # The official LinkedIn embed player URL for a card, when its post URL
    # carries a URN (/feed/update/ links). /posts/ links have none. Read
    # off the message's own reference URL, not the shared row.
    def linkedin_embed_player_url(reference)
      Linkedin::PostUrl.embed_url_for(reference.display_url)
    end

    private
      def request_linkedin_post_fetch(embed)
        return unless embed.needs_fetch?
        return unless (@linkedin_post_fetches ||= Set.new).add?(embed.id)

        LinkEmbed::FetchJob.perform_later(embed) if embed.claim_fetch_request!
      end
  end
end
