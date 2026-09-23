module Linkedin
  module PostsHelper
    # LinkedIn post cards referenced by a message, in link order. Sorted in
    # Ruby off the preloaded references (see LinkEmbedsHelper) so rendering
    # a page of messages costs no per-message queries. Rendering a stale
    # card re-enqueues its fetch, so an expired result refreshes on view.
    def linkedin_post_cards_for(message)
      return [] if message.embeds_suppressed?

      embeds = message.link_embed_references
        .sort_by { |reference| [ reference.position, reference.id ] }
        .map(&:link_embed)
        .select(&:linkedin?)
      embeds.each { |embed| request_linkedin_post_fetch(embed) }
      embeds
    end

    # The official LinkedIn embed player URL for a card, when its post URL
    # carries a URN (/feed/update/ links). /posts/ links have none.
    def linkedin_embed_player_url(embed)
      Linkedin::PostUrl.embed_url_for(embed.display_url)
    end

    private
      def request_linkedin_post_fetch(embed)
        return unless embed.needs_fetch?
        return unless (@linkedin_post_fetches ||= Set.new).add?(embed.id)

        LinkEmbed::FetchJob.perform_later(embed) if embed.claim_fetch_request!
      end
  end
end
