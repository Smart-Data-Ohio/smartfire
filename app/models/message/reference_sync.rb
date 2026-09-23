module Message::ReferenceSync
  # Message permalinks in message text: /rooms/:room_id/@:message_id,
  # absolute or relative, on any host, with any trailing path, query, or
  # fragment. The room segment is informational: the source resolves by
  # message id alone, and the card always shows its true room.
  PATTERN = %r{/rooms/\d+/@(?<id>\d+)\b}

  class << self
    # Reconciles a message's quote references with the permalinks its
    # content currently contains. Idempotent: re-running with unchanged
    # content changes nothing. Links to missing messages, to the message
    # itself, and to system notes create nothing.
    def call(message)
      sources = ::Message.where(id: extract_message_ids(reference_text(message)))
        .where.not(id: message.id).where(system_note: false)

      message.message_references.where.not(referenced_message_id: sources.select(:id)).delete_all

      sources.each do |source|
        message.message_references.find_or_create_by!(referenced_message: source)
      end
    rescue ActiveRecord::RecordNotUnique
      retry
    end

    # Unique quoted message ids in the given text.
    def extract_message_ids(text)
      return [] if text.blank?

      text.to_s.scan(PATTERN).flatten.map(&:to_i).uniq
    end

    private
      def reference_text(message)
        [ message.markdown_source, message.plain_text_body ].compact_blank.join("\n")
      end
  end
end
