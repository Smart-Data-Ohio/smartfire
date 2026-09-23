module RoomsHelper
  # The Stimulus controllers the room page needs before first interaction,
  # preloaded so first paint never waits on a lazy-load waterfall. Only the
  # message list, the composer, and live presence qualify: everything the
  # room renders beyond this (panels, toolbars, pickers, the sidebar frame,
  # tour/help, and the huddle/stage/Drive conditionals) lazy-loads through
  # stimulus-loading on element connect, exactly as it did before
  # preloading existed. Keep this list small on purpose: every entry costs
  # a high-priority request on every room load, and a long list crowds out
  # real fetches (sidebar frame, emoji data) and starves the 250-entry
  # resource-timing buffer. RoomControllerPreloadsTest pins this contract.
  FIRST_PAINT_CONTROLLERS = %w[
    messages maintain_scroll reply composer markdown_editor
    typing_notifications local_time presence
  ].freeze

  def first_paint_controller_preloads
    javascript_module_preload_tag(
      *FIRST_PAINT_CONTROLLERS.map { |name| asset_path("controllers/#{name}_controller.js") }
    )
  end

  def link_to_room(room, **attributes, &)
    link_to room_path(room), **attributes, data: {
      rooms_list_target: "room", room_id: room.id, badge_dot_target: "unread", sorted_list_target: "item"
    }.merge(attributes.delete(:data) || {}), &
  end

  def link_to_edit_room(room, &)
    link_to \
      [ :edit, @room ],
      class: "btn",
      style: "view-transition-name: edit-room-#{@room.id}",
      data: { room_id: @room.id },
      &
  end

  def link_back_to_last_room_visited
    if last_room = last_room_visited
      link_back_to room_path(last_room)
    else
      link_back_to root_path
    end
  end

  def button_to_delete_room(room, url: nil)
    button_to url || room_url(room), method: :delete, class: "btn btn--negative max-width", aria: { label: "Delete #{room.name}" },
        data: { turbo_confirm: "Are you sure you want to delete this room and all messages in it? This can’t be undone." } do
      image_tag("trash.svg", aria: { hidden: "true" }, size: 20) +
      tag.span(room_display_name(room), class: "overflow-ellipsis")
    end
  end

  def button_to_jump_to_newest_message
    tag.button \
        class: "message-area__return-to-latest btn",
        data: { action: "messages#returnToLatest", messages_target: "latest" },
        hidden: true do
      image_tag("arrow-down.svg", aria: { hidden: "true" }, size: 20) +
      tag.span("Jump to newest message", class: "for-screen-reader")
    end
  end

  # The jump pill scrolls to the divider when it is on the page; when the
  # first unread message fell off the page, the pill links to it instead
  # and stays visible, since there is no divider to observe.
  def button_to_jump_to_unread(url: nil)
    label = image_tag("arrow-up.svg", aria: { hidden: "true" }, size: 20) + tag.span("Jump to unread")

    if url
      link_to label, url, id: "jump-to-unread", class: "message-area__jump-to-unread btn"
    else
      tag.button id: "jump-to-unread", class: "message-area__jump-to-unread btn",
          data: { action: "messages#jumpToUnread" }, hidden: true do
        label
      end
    end
  end

  def submit_room_button_tag
    button_tag class: "btn btn--reversed txt-large center", type: "submit" do
      image_tag("check.svg", aria: { hidden: "true" }, size: 20) +
      tag.span("Save", class: "for-screen-reader")
    end
  end

  def composer_form_tag(room, thread: nil, &)
    form_with model: Message.new,
      url: thread ? room_thread_messages_path(room, thread) : room_messages_path(room),
      id: thread ? dom_id(thread, :composer) : "composer",
      namespace: thread ? "thread_#{thread.id}" : nil,
      class: "margin-block flex-item-grow contain",
      data: composer_data_options(room, thread:), &
  end

  # True when the composer offers the enhanced Drive picker: browser
  # sharing is configured and the viewer is a signed-in human member.
  # Calendar/metadata consent is not required; the recipients endpoint
  # re-checks membership and humanity on every request.
  def drive_share_picker_available?
    user = Current.user
    Google::Picker.configured? && user&.active? && !user.bot? && user.agent.nil?
  end

  def room_display_name(room, for_user: Current.user)
    if room.direct?
      room.direct_display_name(for_user: for_user)
    else
      room.name
    end
  end

  # Viewer-neutral room label for shared fragment caches: direct-room
  # names are per-viewer (each member sees the other members' names),
  # so rendering them inside a cached fragment serves one viewer's
  # label to another — and computing them costs a query per room.
  # Direct rooms collapse to a fixed label; named rooms read the
  # preloaded name with no query.
  def viewer_neutral_room_label(room)
    room.direct? ? "a direct message" : room.name
  end

  # Sidebar rows pass their preloaded members so group names never query.
  def direct_room_display_name(room, members:, for_user: Current.user)
    room.direct_display_name(for_user: for_user, members: members)
  end

  def room_kind_label(room)
    if room.direct?
      "Direct message"
    elsif room.voice?
      "Voice channel"
    elsif room.board?
      "Board"
    else
      "Channel"
    end
  end

  private
    def composer_data_options(room, thread: nil)
      message_area_id = thread ? dom_id(thread, :message_area) : "message-area"
      {
        controller: "composer drop-target",
        action: "#{composer_data_actions} turbo:before-fetch-request->composer#prepareRequest messages:recover@window->composer#recover",
        composer_messages_outlet: "##{message_area_id}",
        composer_room_id_value: room.id,
        composer_thread_id_value: thread&.id,
        composer_thread_mode_value: thread.present?,
        composer_slash_commands_url_value: room_slash_commands_path(room),
        composer_slash_commands_value: slash_command_names_for(room),
        composer_slash_commands_list_url_value: autocompletable_slash_commands_path(room_id: room.id, thread_id: thread&.id)
      }
    end

    # Commands the composer intercepts: every built-in plus the room's
    # registered agent commands. The list is rendered once per page
    # load; the composer re-checks the live picker endpoint for words
    # it doesn't know, so commands registered after load still run.
    def slash_command_names_for(room)
      SlashCommands::Registry.all.map(&:name) + room.agent_slash_commands.ordered.pluck(:name)
    end

    def composer_data_actions
      drag_and_drop_actions = "drop-target:drop@window->composer#dropFiles"

      trix_attachment_actions =
        "trix-file-accept->composer#preventAttachment refresh-room:online@window->composer#online"

      remaining_actions =
        "submit->typing-notifications#stop paste->composer#pasteFiles turbo:submit-end->composer#submitEnd refresh-room:offline@window->composer#offline"

      [ drop_target_actions, drag_and_drop_actions, trix_attachment_actions, remaining_actions ].join(" ")
    end
end
