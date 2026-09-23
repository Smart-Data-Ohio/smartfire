module Rooms::InvolvementsHelper
  def turbo_frame_for_involvement_tag(room, &)
    turbo_frame_tag dom_id(room, :involvement), data: {
      controller: "turbo-frame", action: "notifications:ready@window->turbo-frame#load", turbo_frame_url_param: room_involvement_path(room)
    }, &
  end

  # The levels the header overflow menu's notification chooser offers:
  # the same orders the bell cycles through, submitted explicitly instead.
  def involvement_levels_for(room)
    room.direct? ? DIRECT_INVOLVEMENT_ORDER : SHARED_INVOLVEMENT_ORDER
  end

  def short_involvement_label(level)
    SHORT_INVOLVEMENT_LABELS.fetch(level.to_s)
  end

  def involvement_description(level)
    HUMANIZE_INVOLVEMENT.fetch(level.to_s)
  end

  def button_to_change_involvement(room, involvement)
    button_to room_involvement_path(room, involvement: next_involvement_for(room, involvement: involvement)),
      method: :put,
      role: "checkbox", aria: { checked: true, labelledby: dom_id(room, :involvement_label) }, tabindex: 0,
      class: "btn #{involvement}" do
        image_tag("notification-bell-#{involvement}.svg", aria: { hidden: "true" }, size: 20) +
        tag.span(HUMANIZE_INVOLVEMENT[involvement], class: "for-screen-reader", id: dom_id(room, :involvement_label))
    end
  end

  private
    HUMANIZE_INVOLVEMENT = {
      "mentions" => "Notifying about @ mentions",
      "everything" => "Notifying about all messages",
      "muted" => "Muted, notifying only about @ mentions",
      "nothing" => "Notifications are off",
      "invisible" => "Notifications are off and room invisible in sidebar"
    }

    SHARED_INVOLVEMENT_ORDER = %w[ mentions everything muted nothing invisible ]
    DIRECT_INVOLVEMENT_ORDER = %w[ everything muted nothing ]

    SHORT_INVOLVEMENT_LABELS = {
      "mentions" => "Mentions",
      "everything" => "Everything",
      "muted" => "Muted",
      "nothing" => "Off",
      "invisible" => "Invisible"
    }.freeze

    def next_involvement_for(room, involvement:)
      if room.direct?
        DIRECT_INVOLVEMENT_ORDER[DIRECT_INVOLVEMENT_ORDER.index(involvement) + 1] || DIRECT_INVOLVEMENT_ORDER.first
      else
        SHARED_INVOLVEMENT_ORDER[SHARED_INVOLVEMENT_ORDER.index(involvement) + 1] || SHARED_INVOLVEMENT_ORDER.first
      end
    end
end
