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

  # Replies under a message that started a thread, read off the thread's
  # counter (ChannelThread#messages_count), so a page of messages costs no
  # query beyond the preloaded channel_thread. Zero without a thread.
  def thread_reply_count(message)
    message.channel_thread&.messages_count.to_i
  end

  # The indicator's accessible name: what the button does, then the visible
  # count, so the spoken name contains the visible label.
  def thread_indicator_label(count)
    "Open thread, #{pluralize(count, "reply")}"
  end
end
