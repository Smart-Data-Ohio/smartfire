class RoomMailbox < ApplicationMailbox
  BOT_NAME = "Email"

  # One file lands on the message itself (messages carry a single
  # attachment); the rest are named in the body.
  MAX_ATTACHMENT_BYTES = 10.megabytes

  # Posts an email forwarded to a room's secret address as a room
  # message. Anything unauthenticatable is dropped silently: unknown
  # tokens, disabled inbound email, and deleted or direct rooms all end
  # here without a bounce, so the address reveals nothing to probers.
  def process
    return unless Room.inbound_email_enabled?
    return if room.nil?

    room.memberships.find_or_create_by!(user: creator) unless from_member?
    message = room.root_messages.new(creator:, markdown_source: composed_source)
    attach_first_file!(message)
    return if message.markdown_source.blank? && !message.attachment.attached?

    message.save!
    message.process_attachment
    message.broadcast_create
  end

  private
    def room
      @room ||= begin
        token = room_token_from_recipients
        token && Room.alive.without_directs.find_by(inbound_email_token: token)
      end
    end

    def room_token_from_recipients
      mail.recipients.lazy.filter_map { |recipient| recipient.to_s[/room-(.+)@/i, 1] }
        .first.to_s.strip.presence
    end

    # A sender whose address belongs to an active member of the room
    # posts as themselves; everyone else posts as the workspace Email
    # bot with the sender named in the body.
    def creator
      @creator ||= member_sender || email_bot
    end

    def from_member?
      member_sender.present?
    end

    def member_sender
      @member_sender ||= begin
        address = sender_address
        sender = address && User.active.without_bots.where("LOWER(email_address) = ?", address.downcase).first
        sender if sender && room.memberships.exists?(user_id: sender.id)
      end
    end

    def email_bot
      User.active_bots.find_by(name: BOT_NAME) || User.create_bot!(name: BOT_NAME, skip_open_room_grant: true)
    end

    def sender_address
      mail.from&.first.to_s.strip.presence
    end

    def composed_source
      parts = []
      parts << "From #{sender_display}" unless from_member?
      parts << "**#{mail.subject}**" if mail.subject.present?
      parts << body_text if body_text.present?
      parts << attachment_note if attachment_note.present?
      parts.join("\n\n").truncate(Message::Markdown::SOURCE_LIMIT)
    end

    def sender_display
      name = mail[:from]&.display_names&.first.to_s.strip
      address = sender_address
      if name.present? && address.present? && name != address
        "#{name} <#{address}>"
      else
        address.presence || "unknown sender"
      end
    end

    # Plain text preferred; HTML is sanitized and stripped to text. Either
    # way the stored source is plain text, never raw HTML.
    def body_text
      @body_text ||= begin
        text = if mail.multipart?
          mail.text_part ? mail.text_part.decoded : html_to_text(mail.html_part&.decoded.to_s)
        elsif mail.mime_type == "text/html"
          html_to_text(mail.body.decoded.to_s)
        else
          mail.decoded.to_s
        end
        text.gsub(/\r\n?/, "\n").strip.presence
      end
    end

    def html_to_text(html)
      # Tag strippers keep script and style bodies as text, so remove
      # those elements first; the sanitizer below handles the rest.
      pruned = html.to_s.gsub(/<(script|style)\b.*?<\/\1>/mi, "")
      sanitized = Rails::Html::SafeListSanitizer.new.sanitize(pruned)
      ActionView::Base.full_sanitizer.sanitize(sanitized).gsub(/[ \t]+\n/, "\n").strip
    end

    def attachable_files
      @attachable_files ||= mail.attachments.select do |attachment|
        attachment.filename.present? && attachment.decoded.bytesize <= MAX_ATTACHMENT_BYTES
      end
    end

    def attach_first_file!(message)
      file = attachable_files.first
      return if file.nil?

      message.attachment.attach(
        io: StringIO.new(file.decoded),
        filename: file.filename.to_s,
        content_type: file.mime_type.to_s.presence || "application/octet-stream"
      )
    end

    # Names every attached and skipped file so nothing arrives silently
    # missing: skipped files name the reason (over the size limit).
    def attachment_note
      @attachment_note ||= begin
        names = mail.attachments.filter_map do |attachment|
          filename = attachment.filename.to_s.strip.presence
          next if filename.nil?
          next filename if attachable_files.include?(attachment)

          "#{filename} (not attached: over the #{MAX_ATTACHMENT_BYTES / 1.megabyte} MB limit)"
        end
        names.any? ? "Attached files: #{names.join(", ")}" : nil
      end
    end
end
