module ActivityItemsHelper
  def activity_item_source_path(item)
    source = reminder_message_source(item.source)
    return activity_items_path unless source

    case source
    when Message
      if source.thread_message?
        room_path(source.room, thread: source.thread_id, message_id: source.id)
      else
        room_at_message_path(source.room, source)
      end
    when WorkThreadEvent
      thread = source.thread
      thread ? room_path(thread.room, thread: thread.id) : activity_items_path
    when BoardSlaNudge
      thread = source.channel_thread
      thread ? room_path(thread.room, thread: thread.id) : activity_items_path
    when HuddleGrant
      source.room ? room_path(source.room) : activity_items_path
    when Event
      source.room ? room_event_path(source.room, source) : activity_items_path
    when AgentApproval
      agent_approvals_path(source.agent)
    when ScheduledMessage
      scheduled_messages_path
    else
      activity_items_path
    end
  end

  def activity_item_event_label(item)
    case item.event_type
    when "mention"
      "Mention"
    when "reply"
      "Reply"
    when "keyword_alert"
      "Keyword alert"
    when "thread_activity"
      "Followed thread"
    when "work_assignment"
      "Work assignment"
    when "work_update"
      "Work update"
    when "work_sla"
      "SLA breach"
    when "huddle_started"
      "Incoming huddle"
    when "huddle_missed"
      "Missed huddle"
    when "event_invitation"
      "Event invitation"
    when "event_update"
      "Event update"
    when "event_cancelled"
      "Event cancelled"
    when "event_reminder"
      "Event reminder"
    when "pr_review_request"
      "Review requested"
    when "agent_approval_request"
      "Approval request"
    when "message_reminder"
      "Reminder"
    when "scheduled_message_dropped"
      "Scheduled message not sent"
    else
      item.event_type.humanize
    end
  end

  def activity_item_source_label(item)
    source = reminder_message_source(item.source)
    return "Unavailable source" unless source

    case source
    when Message
      if source.thread_message?
        "#{room_display_name(source.room)} · #{source.thread.name}"
      else
        room_display_name(source.room)
      end
    when WorkThreadEvent
      source.thread ? "#{room_display_name(source.thread.room)} · #{source.thread.name}" : "Unavailable thread"
    when BoardSlaNudge
      thread = source.channel_thread
      thread ? "#{room_display_name(thread.room)} · #{thread.name}" : "Unavailable thread"
    when HuddleGrant
      source.room ? room_display_name(source.room) : "Unavailable room"
    when Event
      source.room ? "#{room_display_name(source.room)} · #{source.title}" : source.title
    when AgentApproval
      agent = source.agent
      room = source.room
      base = agent ? agent.user.name : "Agent"
      room ? "#{base} · #{room_display_name(room)}" : base
    when ScheduledMessage
      room = source.room
      room ? room_display_name(room) : "Unavailable room"
    else
      source.class.name.humanize
    end
  end

  def activity_item_source_body(item)
    source = reminder_message_source(item.source)
    return "This source is no longer available." unless source

    case source
    when Message
      if item.event_type == "message_reminder"
        "You asked to be reminded about this message: #{source.plain_text_body}"
      else
        source.plain_text_body
      end
    when WorkThreadEvent
      changes = []
      if source.status_changed?
        changes << "Status: #{activity_item_work_status_label(source.from_status)} → #{activity_item_work_status_label(source.to_status)}"
      end
      if source.owner_changed?
        changes << "Owner: #{source.from_owner_name.presence || "unassigned"} → #{source.to_owner_name.presence || "unassigned"}"
      end
      changes.presence&.to_sentence || "Work thread updated"
    when BoardSlaNudge
      status = ChannelThread::WORK_STATUS_LABELS.fetch(source.work_status, source.work_status.to_s.humanize)
      waited = source.waited_minutes
      age = waited >= 60 ? "#{(waited / 60.0).round(1)} hours" : "#{waited} minutes"
      if source.stage == "escalation"
        "Escalated: sitting in #{status} for #{age}"
      else
        "Sitting in #{status} for #{age}"
      end
    when HuddleGrant
      caller = source.user&.name || "Someone"
      if item.event_type == "huddle_missed"
        "You missed a huddle from #{caller}"
      else
        "#{caller} started a huddle"
      end
    when Event
      activity_item_event_body(item)
    when AgentApproval
      source.summary.to_s
    when ScheduledMessage
      if source.drop_reason.present?
        "Your scheduled message was not sent (#{source.drop_reason}): #{source.markdown_source}"
      else
        "You no longer have access to this room, so your scheduled message was not sent: #{source.markdown_source}"
      end
    else
      "Source updated"
    end
  end

  def activity_item_source_author(item)
    source = reminder_message_source(item.source)
    case source
    when Message
      source.creator&.name
    when WorkThreadEvent
      source.actor&.name || "Work thread"
    when HuddleGrant
      source.user&.name
    when Event
      source.organizer&.name
    when AgentApproval
      source.agent&.user&.name
    when ScheduledMessage
      source.user&.name
    end
  end

  # Reminder items are sourced on the saved item (so firing never
  # converts a mention or reply item for the message); they render
  # through the saved message, like message-sourced items do.
  def reminder_message_source(source)
    source.is_a?(SavedItem) ? source.message : source
  end

  def activity_item_work_status_label(status)
    status.present? ? status.humanize : "None"
  end

  def activity_item_event_body(item)
    event = item.source
    start = event.starts_at.in_time_zone(event.time_zone).strftime("%B %-d, %Y at %-I:%M %p %Z")

    case item.event_type
    when "event_invitation"
      if event.recurrence_rule.present? && event.recurrence_until.present?
        "You are invited: #{start} (repeats #{Event::Recurrence.phrase(event.recurrence_rule)} until #{event.recurrence_until.strftime("%B %-d, %Y")})."
      else
        "You are invited: #{start}."
      end
    when "event_update"
      "The time changed: #{start}."
    when "event_cancelled"
      "This event was cancelled."
    when "event_reminder"
      if event.venue.present?
        "Starts in 15 minutes: #{event.title} in #{event.venue.name}."
      else
        "Starts in 15 minutes: #{event.title}."
      end
    else
      "Event updated: #{start}."
    end
  end
end
