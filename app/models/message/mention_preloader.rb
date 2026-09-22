module Message::MentionPreloader
  MENTION_SGID_PATTERN = /sgid="([^"]+)"/

  class << self
    # Loads every user mentioned across a page of messages in one query, so
    # rendering (presentation partials, plain-text bodies) resolves mentions
    # from memory instead of one query each. Only bodies already loaded are
    # scanned, so this never queries per message itself; messages render
    # correctly without it through the usual per-mention lookup.
    def preload_for(messages)
      ids = mentioned_user_ids(Array(messages)) - store.keys
      if ids.any?
        User.where(id: ids).includes(:avatar_attachment).each do |user|
          store[user.id] = user
        end
      end
      messages
    end

    # The preloaded user behind an attachment node, if any. Used by the
    # ActionText attachment patch; falls through to the normal lookup
    # (verified, then expired-signature) when nothing was preloaded.
    def preloaded_user_for(node)
      return if store.empty?

      sgid = node["sgid"]
      return if sgid.blank?

      user_id = user_id_for_sgid(sgid)
      user_id && store[user_id]
    end

    def user_id_for_sgid(sgid)
      gid = gid_uri_for_sgid(sgid)&.then { |uri| GlobalID.parse(uri) }
      gid.model_id.to_i if gid&.model_name == "User"
    rescue StandardError
      nil
    end

    # The gid URI carried in an attachment sgid, without verifying the
    # signature — the same trust the expired-signature fallback already
    # extends to User mentions (see action_text_attachables.rb).
    def gid_uri_for_sgid(sgid)
      message = sgid&.split("--")&.first
      return if message.blank?

      encoded_message = JSON.parse(decode_base64(message))

      if data = encoded_message.dig("_rails", "data")
        data
      elsif data = encoded_message.dig("_rails", "message")
        # Rails 7 used an older format of GID that serialized the payload using Marshall
        # Since we intentionally skip signature verification, we can't safely unmarshal the data
        # To work around this, we manually extract the GID from the marshaled data
        decode_base64(data).match(%r{(gid://campfire/[^/]+/\d+)})&.to_s.presence
      end
    end

    private
      # Request-scoped: CurrentAttributes resets after each request, so a
      # page's mentions never leak into the next request on the same thread.
      def store
        Current.mentioned_users_by_id ||= {}
      end

      def mentioned_user_ids(messages)
        messages.flat_map do |message|
          preloaded_bodies_for(message).flat_map do |content|
            # to_html serializes the stored nodes; to_s would render the
            # attachments and resolve every mention with a query.
            content.to_html.scan(MENTION_SGID_PATTERN).filter_map { |match| user_id_for_sgid(match.first) }
          end
        end.uniq
      end

      def preloaded_bodies_for(message)
        bodies = []
        bodies << message.body.body if message_body_preloaded?(message)
        if message.association(:reply_to_message).loaded? && (source = message.reply_to_message)
          bodies << source.body.body if message_body_preloaded?(source)
        end
        bodies
      end

      # Attachment-only messages have no rich text body to scan.
      def message_body_preloaded?(message)
        message.association(:rich_text_body).loaded? && message.body.body.present?
      end

      def decode_base64(message)
        Base64.strict_decode64(message)
      rescue => _e
        Base64.urlsafe_decode64(message)
      end
  end
end
