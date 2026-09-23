module Fizzy
  # Reconciles a message's card references with the URLs its content
  # currently contains, and warms the author's own cache for every newly
  # referenced or stale card so their view fills in without a reload.
  # Other viewers fetch with their own token when their card frame loads.
  # Idempotent: re-running with unchanged content enqueues nothing new.
  module CardReferenceSync
    class << self
      def call(message)
        pairs = CardUrl.extract(reference_text(message))

        cards = pairs.map do |ref|
          Card.for_reference(account_id: ref.account_id, number: ref.number)
        end

        message.fizzy_card_references
          .where.not(fizzy_card_id: cards.map(&:id))
          .delete_all

        cards.each do |card|
          reference = message.fizzy_card_references.find_or_create_by!(card: card)
          warm_author_cache(message, card) if reference.previously_new_record? || author_cache_stale?(message, card)
        end
      rescue ActiveRecord::RecordNotUnique
        retry
      end

      private
        def reference_text(message)
          [ CardUrl.non_code_text(message.body.body&.to_html), message.forward_note ].compact_blank.join("\n")
        end

        def warm_author_cache(message, card)
          author = message.creator
          return unless author&.fizzy_connected_account&.usable?

          cache = CardCache.for_viewer(card: card, user: author)
          return unless cache.stale?
          return unless cache.claim_fetch_request!

          begin
            Fizzy::FetchCardJob.perform_later(card, author)
          rescue Redis::BaseError, RedisClient::Error => error
            cache.release_fetch_request!
            Rails.logger.warn "Skipping Fizzy warmup enqueue for card #{card.id}: #{error.class}"
          end
        end

        def author_cache_stale?(message, card)
          author = message.creator
          return false unless author

          cache = CardCache.find_by(fizzy_card_id: card.id, user_id: author.id)
          cache.nil? || cache.stale?
        end
    end
  end
end
