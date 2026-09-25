module MessageThreadsHelper
  def thread_panel_data(room)
    {
      room_id: room.id,
      threads_url: room_threads_path(room, format: :json),
      create_url: room_threads_path(room, format: :json),
      channel_name: room_display_name(room, for_user: nil),
      default_avatar_url: asset_path("default-avatar.svg")
    }
  end

  def thread_panel_url(room, thread)
    room_thread_path(room, thread, format: :json)
  end

  # Falls back to a single count query when no bulk `counts` hash was
  # computed for the page (low-traffic render sites: search, a thread's
  # own conversation, board posts).
  def thread_reply_count(message, counts: nil)
    thread = message.channel_thread
    return 0 if thread.blank?

    counts ? counts.fetch(thread.id, 0) : thread.message_count
  end
end
