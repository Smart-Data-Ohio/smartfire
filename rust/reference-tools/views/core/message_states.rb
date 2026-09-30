# Plain model facts, and actual output from each branch of our message partial.
def message_view_facts(view, message, show_room_icon: false)
  author = ->(u) { { id: u.id, name: u.name, title: u.title, avatar_url: view.fresh_user_avatar_path(u) } }
  icon_facts = ->(icon) {
    if icon&.emoji?
      { "Emoji" => { title: icon.title, character: icon.character } }
    elsif icon && (url = Icons.image_url_for(icon))
      { "Image" => { title: icon.title, url: url, brand: icon.brand? } }
    end
  }
  source = message.reply_to_message
  {
    id: message.id, client_message_id: message.client_message_id, room_id: message.room_id,
    room_name: view.room_display_name(message.room, for_user: nil), creator: author.call(message.creator),
    created_at: message.created_at.iso8601(6), updated_at: message.updated_at.iso8601(6),
    all_emoji: message.plain_text_body.all_emoji?, content: message_content_facts(view, message),
    boosts: message.ordered_boosts.map do |boost|
      reaction = if Boost.reaction?(boost.content)
        { title: view.reaction_title(boost.content), icon: icon_facts.call(view.send(:shortcode_icon, boost.content)), icon_alt: (view.send(:shortcode_icon, boost.content)&.then { |icon| ":#{icon.name}:" }) }
      end
      { id: boost.id, updated_at: boost.updated_at.iso8601(6), message_id: message.id,
        content: boost.content, all_emoji: boost.content.all_emoji?, booster: author.call(boost.booster), reaction: reaction }
    end,
    details: {
      thread_id: message.thread_id, system_note: message.system_note?, action: message.action?,
      edited_at: message.edited_at&.iso8601, streaming: message.streaming?, markdown: message.markdown?,
      pinned: view.message_pinned?(message), reply_count: view.thread_reply_count(message),
      reply: message.reply? ? { source: source && { id: source.id, author: source.creator.name, plain_text: source.plain_text_body, url: view.message_link_url(source) } } : nil,
      forwarded: message.forwarded?, forward_note: message.forward_note,
      drive_urls: message.drive_attachments.map(&:url),
      room_icon: show_room_icon ? icon_facts.call(Icons.find(message.room.icon_name)) : nil,
      agent_steps: message.agent_steps.map { |step| step.attributes.symbolize_keys.slice(:name, :status, :duration_ms, :input_summary, :output_summary) },
      poll: message.poll && {
        id: message.poll.id, room_id: message.room_id, anonymous: message.poll.anonymous?, multiple: message.poll.multiple?,
        closed: message.poll.closed?, closes_at: message.poll.closes_at&.iso8601,
        options: message.poll.poll_options.map { |option| { id: option.id, label: option.label } },
        votes: message.poll.poll_votes.map { |vote| { option_id: vote.poll_option_id, user_id: vote.user_id, user_name: vote.user&.name } }
      }
    }
  }
end

def message_content_facts(view, message)
  return { type: "text", html: view.message_presentation(message) } unless message.attachment.attached?

  attachment = message.attachment
  preview = if attachment.video?
    { type: "video", poster_url: view.url_for(attachment.preview(format: :webp, resize_to_limit: [Message::THUMBNAIL_MAX_WIDTH, Message::THUMBNAIL_MAX_HEIGHT])) }
  elsif attachment.previewable? || attachment.variable?
    { type: "image", thumb_url: view.polymorphic_url(attachment.representation(:thumb), only_path: true) }
  else
    { type: "file" }
  end
  { type: "attachment", filename: attachment.filename.to_s, filename_base: attachment.filename.base.to_s,
    blob_path: view.rails_blob_path(attachment), download_path: view.rails_blob_path(attachment, disposition: "attachment", only_path: true),
    preview: preview, width: attachment.metadata[:width], height: attachment.metadata[:height] }
end

def generate_message_states(goldens, view)
  poll_state = ->(message, anonymous, multiple, closed, closes_at) {
    poll = Poll.new(id: 900, message: message, anonymous: anonymous, multiple: multiple,
      closed_at: closed ? Time.current : nil, closes_at: closes_at)
    options = [PollOption.new(id: 901, label: "First <&>", position: 0), PollOption.new(id: 902, label: "Second", position: 1), PollOption.new(id: 903, label: "Zero votes", position: 2)]
    votes = [[901, "Jason"], [901, "David"], [902, "JZ"]].map { |id, name| PollVote.new(poll_option_id: id, user: User.find_by!(name: name)) }
    poll.association(:poll_options).target = options; poll.association(:poll_options).loaded!
    poll.association(:poll_votes).target = votes; poll.association(:poll_votes).loaded!
    message.association(:poll).target = poll; message.association(:poll).loaded!
  }
  states = {
    "steps" => ->(message) {
      steps = [ ["running", nil, nil, nil], ["done", 0, "In <&>\nnext", "Out \"quoted\""], ["failed", 999, " ", nil], ["done", 1500, nil, "Summary"] ].map do |status, duration, input, output|
        AgentStep.new(name: "Step <&>", status: status, duration_ms: duration, input_summary: input, output_summary: output)
      end
      message.association(:agent_steps).target = steps; message.association(:agent_steps).loaded!
    },
    "poll_open" => ->(m) { poll_state.call(m, false, false, false, nil) },
    "poll_multiple_anonymous" => ->(m) { poll_state.call(m, true, true, false, Time.current + 3600) },
    "poll_closed" => ->(m) { poll_state.call(m, false, true, true, nil) },
    "poll_due_anonymous" => ->(m) { poll_state.call(m, true, false, false, Time.current - 3600) },
    "plain" => ->(_) {},
    "markdown_empty" => ->(m) { m.markdown_source = "" },
    "room_icon" => ->(m) { m.room.icon_name = "github" },
    "edited_action" => ->(m) { m.action = true; m.edited_at = Time.utc(2026, 2, 10, 11, 30) },
    "streaming" => ->(m) { m.streaming = true },
    "system_note" => ->(m) { m.system_note = true },
    "reply" => ->(m) { m.reply_to_message = Message.find_by!(client_message_id: "0002") },
    "deleted_reply" => ->(m) { m.reply_to_message_id = nil; m.reply_target_deleted_at = Time.current },
    "forwarded" => ->(m) { m.forwarded_at = Time.current; m.forward_note = "Note <&> \"quoted\"" },
    "thread" => ->(m) { m.thread = ChannelThread.new(id: 900, room_id: m.room_id) },
    "pinned_drive_replies" => ->(m) {
      m.association(:message_pins).target = [MessagePin.new(message: m)]; m.association(:message_pins).loaded!
      m.association(:drive_attachments).target = [DriveAttachment.new(file_id: "abc_123-xyz")]; m.association(:drive_attachments).loaded!
      m.association(:channel_thread).target = ChannelThread.new(id: 900, messages_count: 1); m.association(:channel_thread).loaded!
    },
    "reactions" => ->(m) {
      boosts = [["👍", "David"], ["👍", "Jason"], ["👍", "David"], ["❤️", "Jason"], [":github:", "David"], ["Hello", "David"]].each_with_index.map do |(content, name), i|
        Boost.new(id: 900 + i, message: m, booster: User.find_by!(name: name), content: content, created_at: m.created_at + i, updated_at: m.updated_at)
      end
      m.association(:boosts).target = boosts; m.association(:boosts).loaded!
    }
  }
  goldens["helpers"]["review"]["message_states"] = states.map do |name, change|
    message = Message.with_rendering_details.find_by!(client_message_id: "0001")
    change.call(message)
    Rails.cache.clear
    { name: name, message: message_view_facts(view, message, show_room_icon: name == "room_icon"), html: render_with(user: user("david@37signals.com"), partial: "messages/message", locals: { message: message, show_room_icon: name == "room_icon" }) }
  end
  puts "Rails message states: #{states.size} complete message trees"
end
