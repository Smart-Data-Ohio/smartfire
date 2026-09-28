module SlackImportsHelper
  # Renders a dry-run sample's converted markdown through the same path
  # as chat messages, so the plan shows what the import will produce.
  # Mentions resolve against no room here and stay literal tokens.
  def slack_sample_html(markdown)
    content = ActionText::Content.new(Message::Markdown.render(markdown.to_s, room: Room.new))
    markdown_message_presentation(content)
  end

  def slack_run_title(run)
    "#{run.kind.humanize} #{run.mode.humanize.downcase} ##{run.id}"
  end

  def slack_conversation_type_label(type)
    {
      "public_channel" => "Public channel",
      "private_channel" => "Private channel",
      "im" => "Direct message",
      "mpim" => "Group DM"
    }.fetch(type.to_s, type.to_s.humanize)
  end
end
