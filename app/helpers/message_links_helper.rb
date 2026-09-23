module MessageLinksHelper
  # A message's quote references in creation order. Reads the preloaded
  # with_rendering_details association on message pages; single-message
  # renders (broadcasts, permalinks) load the list with its sources in
  # one grouped set of queries instead.
  def message_quote_references(message)
    if message.association(:message_references).loaded?
      message.message_references.sort_by(&:id)
    else
      message.message_references
        .includes(referenced_message: [ :room, :rich_text_body, { attachment_attachment: :blob }, { creator: :avatar_attachment } ])
        .order(:id).to_a
    end
  end

  # Frame ids for cross-room quote cards. The cards container renders the
  # empty frame with this id; the quote endpoint recomputes the same id
  # from the reference so the loaded frame replaces the placeholder.
  def message_link_frame_id(reference)
    dom_id(reference, :message_link_card)
  end

  # Whether the viewer may see the quoted source: its room must be alive
  # and the viewer a member. Thread sources follow the same rule, since
  # every room member may open every thread in the room.
  def message_quote_visible_to?(source, user)
    user.present? && !source.system_note? &&
      user.rooms.where(id: source.room_id).exists?
  end

  # The newest edit stamp over a message's quoted sources, for the
  # fragment cache key. edited_at rides beside updated_at because a
  # legacy body edit rewrites only the rich-text row, leaving the source
  # row untouched. Reads the preloaded association on message pages.
  def message_quote_stamp(message)
    quoted_sources(message).flat_map { |source| [ source.updated_at, source.edited_at ] }.compact.max
  end

  # The names a message's quote cards show, for the fragment cache key.
  # Renames touch neither the quoting message nor the quoted source, so
  # without this a cached card would keep the old author or room label
  # indefinitely. A digest rather than updated_at maxima: rooms touch
  # on every post, which would bust every quoting fragment constantly.
  # Reads the preloaded associations on message pages.
  def message_quote_names_digest(message)
    names = quoted_sources(message).map { |source| [ source.creator.name, source.room.name ] }
    Digest::SHA256.hexdigest(names.sort.inspect) if names.any?
  end

  private
    def quoted_sources(message)
      references = if message.association(:message_references).loaded?
        message.message_references
      else
        message.message_references.to_a
      end

      references.filter_map(&:referenced_message)
    end
end
