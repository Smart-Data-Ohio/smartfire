class RoomMailbox < ApplicationMailbox
  BOT_NAME = "Email"

  # One file lands on the message itself (messages carry a single
  # attachment); the rest are named in the body.
  MAX_ATTACHMENT_BYTES = 10.megabytes

  # At most this many emailed messages per room per hour, so one
  # address cannot flood a room.
  MAX_EMAILS_PER_ROOM_PER_HOUR = 30

  # Only these attachment types ever land on the message: images,
  # PDFs, plain text, and office documents. Everything else is named
  # in the body with the reason, never attached.
  ALLOWED_ATTACHMENT_PREFIXES = %w[
    image/
    text/
    application/vnd.openxmlformats-officedocument
    application/vnd.oasis.opendocument
  ].freeze
  ALLOWED_ATTACHMENT_TYPES = %w[
    application/pdf
    application/rtf
    application/msword
    application/vnd.ms-excel
    application/vnd.ms-powerpoint
  ].freeze

  # Posts an email forwarded to a room's secret address as a room
  # message. Anything unauthenticatable is dropped silently: unknown
  # tokens, disabled inbound email, and deleted, direct, or board rooms
  # all end here without a bounce, so the address reveals nothing to
  # probers.
  def process
    return unless Room.inbound_email_enabled?
    return if room.nil?
    return if rate_limited?

    room.memberships.find_or_create_by!(user: creator) unless from_member?
    message = room.root_messages.new(creator:, markdown_source: composed_source)
    attach_first_file!(message)
    return if nothing_to_say? && !message.attachment.attached?

    message.save!
    message.process_attachment
    message.broadcast_create
    Message::BotWebhookFanout.deliver_for(message)
  end

  private
    def room
      @room ||= begin
        token = room_token_from_recipients
        found = token && Room.alive.without_directs.find_by(inbound_email_token: token)
        found if found&.emailable?
      end
    end

    def room_token_from_recipients
      mail.recipients.lazy.filter_map { |recipient| recipient.to_s[/room-(.+)@/i, 1] }
        .first.to_s.strip.presence
    end

    # Hour-bucketed per-room limit. Backed by Rails.cache like the agent
    # API throttle, so it only bites when a real cache store is
    # configured. Over-limit mail is dropped silently: it reached a
    # valid address, so no bounce that confirms anything.
    def rate_limited?
      bucket = Time.current.strftime("%Y%m%d%H")
      count = Rails.cache.increment(
        "room_inbound_email:#{room.id}:#{bucket}", 1, expires_in: 1.hour + 5.minutes
      ).to_i
      count > MAX_EMAILS_PER_ROOM_PER_HOUR
    end

    # A sender whose address belongs to an active member of the room
    # posts as themselves, but only when the configured relay's own
    # Authentication-Results show SPF, DKIM or DMARC passing for the
    # From domain; everyone else — including a spoofed member From with
    # no pass — posts as the workspace Email bot with the sender named
    # in the body.
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
        sender if sender && room.memberships.exists?(user_id: sender.id) && authenticated_sender?(address)
      end
    end

    # True when the relay reports SPF, DKIM or DMARC passing for the
    # From domain in Authentication-Results (RFC 8601, stamped by the
    # relay at receive time):
    # https://www.rfc-editor.org/rfc/rfc8601.html
    #
    # Only the topmost header field whose authserv-id matches the
    # configured relay (INBOUND_EMAIL_AUTHSERV_ID) is trusted; a
    # sender-forged field carries another id and is ignored. While
    # the id is unconfigured, or no field matches, nothing verifies.
    # The relay must stamp its own header (see
    # docs/email-to-room.md). A pass only counts when the method's
    # own domain property matches the From domain: header.d for
    # DKIM, header.from for DMARC, smtp.mailfrom for SPF. An SPF
    # pass on smtp.helo alone never verifies: the HELO name is not
    # the sender.
    def authenticated_sender?(address)
      domain = address.to_s.split("@").last.to_s.downcase
      return false if domain.blank?

      results = trusted_authentication_results
      return false if results.nil?

      authentication_passed_for?(results, domain)
    end

    def trusted_authentication_results
      authserv_id = Room.inbound_email_authserv_id
      return nil if authserv_id.nil?

      authentication_results.find do |results|
        results.split(";").first.to_s.strip.casecmp?(authserv_id)
      end
    end

    def authentication_results
      mail.header.fields
        .select { |field| field.name.casecmp?("Authentication-Results") }
        .map { |field| field.value.to_s }
    end

    def authentication_passed_for?(results, domain)
      clauses = results.split(";").map(&:strip)
      clauses.shift # leading authserv-id carries no result
      clauses.any? do |clause|
        method, rest = clause.split("=", 2).map { |part| part.to_s.strip.downcase }
        next false unless rest.to_s.split(/[\s(]/, 2).first == "pass"

        case method
        when "dkim"
          clause_domains(clause, %w[ header.d ]).include?(domain)
        when "dmarc"
          clause_domains(clause, %w[ header.from ]).include?(domain)
        when "spf"
          clause_domains(clause, %w[ smtp.mailfrom ]).include?(domain)
        else
          false
        end
      end
    end

    def clause_domains(clause, properties)
      pattern = properties.map { |property| Regexp.escape(property) }.join("|")
      clause.scan(/(?:#{pattern})=([^\s;()]+)/i)
        .flatten.map { |value| value.downcase.sub(/\A@/, "").split("@").last }
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

    # Empty mail posts nothing, even though the composed source would
    # carry a "From ..." line for bot-posted mail: only the subject,
    # body, and file notes count as something to say.
    def nothing_to_say?
      mail.subject.blank? && body_text.blank? && attachment_note.blank?
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
        attachment.filename.present? && attachment_verdict(attachment) == :ok
      end
    end

    # One verdict per attachment, memoized so a part is decoded at most
    # once: too big (by estimate, then exactly), a disallowed type, or
    # ok. Size is checked first so an oversized part of any type names
    # the limit.
    def attachment_verdict(attachment)
      @attachment_verdicts ||= {}
      @attachment_verdicts[attachment.object_id] ||= begin
        if !estimated_size_ok?(attachment) || !decoded_size_ok?(attachment)
          :too_big
        elsif !allowed_attachment_type?(attachment)
          :disallowed_type
        else
          :ok
        end
      end
    end

    def allowed_attachment_type?(attachment)
      type = attachment.mime_type.to_s.downcase
      ALLOWED_ATTACHMENT_TYPES.include?(type) ||
        ALLOWED_ATTACHMENT_PREFIXES.any? { |prefix| type.start_with?(prefix) }
    end

    # Estimated from the encoded MIME part before decoding: base64
    # inflates 4:3, so the raw part size bounds the decoded size without
    # paying for the decode of a huge part.
    def estimated_size_ok?(attachment)
      attachment.body.raw_source.to_s.bytesize * 3 / 4 <= MAX_ATTACHMENT_BYTES
    end

    # Exact check once decoded: unencoded (7bit) parts are not inflated,
    # so the estimate above cannot reject them.
    def decoded_size_ok?(attachment)
      attachment.decoded.bytesize <= MAX_ATTACHMENT_BYTES
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
    # missing: skipped files name the reason (over the size limit, or a
    # type outside the allowlist).
    def attachment_note
      @attachment_note ||= begin
        names = mail.attachments.filter_map do |attachment|
          filename = attachment.filename.to_s.strip.presence
          next if filename.nil?
          next filename if attachable_files.include?(attachment)

          "#{filename} (not attached: #{skip_reason(attachment)})"
        end
        names.any? ? "Attached files: #{names.join(", ")}" : nil
      end
    end

    def skip_reason(attachment)
      if attachment_verdict(attachment) == :too_big
        "over the #{MAX_ATTACHMENT_BYTES / 1.megabyte} MB limit"
      else
        "file type not allowed"
      end
    end
end
