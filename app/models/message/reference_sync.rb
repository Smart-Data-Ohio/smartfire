module Message::ReferenceSync
  # Message permalinks in message text: /rooms/:room_id/@:message_id,
  # absolute or relative, on any host, with any trailing path, query, or
  # fragment. The room segment is informational: the source resolves by
  # message id alone, and the card always shows its true room.
  PATTERN = %r{/rooms/\d+/@(?<id>\d+)\b}

  MAX_PER_MESSAGE = 10

  # Block-level tags whose boundaries separate words when flattening
  # HTML to text. fragment.text joins across them, which would glue a
  # URL onto adjacent prose (.../@12thanks) and hide it from
  # extraction. code and pre are absent: they are removed beforehand.
  BLOCK_TAGS = %w[
    address article aside blockquote dd dialog div dl dt fieldset
    figcaption figure footer form h1 h2 h3 h4 h5 h6 header hgroup hr
    li main nav ol p section table td th tr ul
  ].freeze

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

    # Unique quoted message ids in the given text, in order of
    # appearance, capped at MAX_PER_MESSAGE.
    def extract_message_ids(text)
      return [] if text.blank?

      ids = []
      text.to_s.scan(PATTERN) do
        id = Regexp.last_match[:id].to_i
        next if ids.include?(id)

        ids << id
        break if ids.size >= MAX_PER_MESSAGE
      end
      ids
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

    private
      def reference_text(message)
        [ non_code_text(message.body.body&.to_html), message.forward_note ].compact_blank.join("\n")
      end
  end
end
