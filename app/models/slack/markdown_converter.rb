module Slack
  # Pure mrkdwn-to-Smartfire-markdown converter. Takes one Slack message
  # hash plus a lookup of Slack user id to Smartfire display name, and
  # returns converted markdown plus flags the writer turns into issues.
  #
  # The output must render through Message::Markdown (Commonmarker with
  # autolink, strikethrough, tables and tasklists, then mention-token and
  # :shortcode: expansion), so conversions target exactly that pipeline:
  # mentions become @[Full Name] tokens, everything else plain markdown.
  class MarkdownConverter
    Result = Data.define(:markdown, :truncated, :files_linked)

    TRUNCATION_OMISSION = "… (import truncated)"

    # Matches fenced blocks and inline spans so emphasis, link and mention
    # rewriting leaves code untouched. Slack escapes &, <, > inside code
    # too, so unescaping runs before this split.
    CODE_PATTERN = /(```.*?```|`[^`\n]*`)/m
    # Slack *bold*, _italic_ and ~strike~ never span newlines. The italic
    # pattern needs word boundaries so snake_case stays literal.
    BOLD_PATTERN = /(?<!\*)\*([^*\n]+)\*(?!\*)/
    ITALIC_PATTERN = /(?<!\w)_([^_\n]+)_(?!\w)/
    STRIKE_PATTERN = /~([^~\n]+)~/
    SKIN_TONE_PATTERN = /::skin-tone-\d+/

    class << self
      # message is one history/replies hash. users maps Slack user ids (and
      # bot keys, see UserMapper.bot_key) to Smartfire display names, with
      # nil for unknown users.
      def convert(message, users: {})
        text = unescape(message["text"].to_s)
        text = append_attachments(text, message)
        converted = convert_segments(text, users)
        converted = apply_me_message(converted, message)
        converted, files_linked = append_files(converted, message)
        converted, truncated = truncate(converted)

        Result.new(markdown: converted, truncated:, files_linked:)
      end

      private
        def unescape(text)
          text.gsub("&amp;", "&").gsub("&lt;", "<").gsub("&gt;", ">")
        end

        # Bot and unfurl payloads live in `attachments`, outside `text`.
        # Quote them in when the message would otherwise be empty or when
        # the author is a bot, whose text usually only summarizes them.
        def append_attachments(text, message)
          attachments = Array(message["attachments"]).filter_map do |attachment|
            attachment_text = [ attachment["pretext"], attachment["text"], attachment["fallback"] ]
              .map { |part| unescape(part.to_s).strip }.reject(&:blank?).uniq.join("\n")
            attachment_text if attachment_text.present?
          end
          return text if attachments.empty?
          return text unless text.strip.blank? || bot_message?(message)

          [ text.strip.presence, *attachments.map { |quoted| quote(quoted) } ]
            .compact.join("\n\n")
        end

        def bot_message?(message)
          message["subtype"] == "bot_message" || message["bot_id"].present?
        end

        def quote(text)
          text.lines.map { |line| line.strip.present? ? "> #{line.strip}" : ">" }.join("\n")
        end

        def convert_segments(text, users)
          segments = text.split(CODE_PATTERN)
          segments.each_with_index.map do |segment, index|
            index.odd? ? segment : convert_prose(segment, users)
          end.join
        end

        def convert_prose(text, users)
          text = convert_links(text)
          text = convert_mentions(text, users)
          text = convert_emphasis(text)
          text = convert_bullets(text)
          strip_skin_tones(text)
        end

        def convert_links(text)
          # <mailto:addr|label> and <mailto:addr> first: the generic URL
          # patterns below would otherwise swallow the mailto form.
          text = text.gsub(/<mailto:([^>|]+)\|([^>]+)>/) { "[#{$2}](mailto:#{$1})" }
          text = text.gsub(/<mailto:([^>]+)>/) { $1 }
          text = text.gsub(/<(https?:\/\/[^|>]+)\|([^>]+)>/) { "[#{$2}](#{$1})" }
          text.gsub(/<(https?:\/\/[^>]+)>/) { $1 }
        end

        def convert_mentions(text, users)
          text = text.gsub(/<@([A-Z0-9]+)(?:\|([^>]+))?>/) do
            name = users[$1]
            if name.present? && !name.match?(/[\[\]\r\n]/)
              "@[#{name}]"
            else
              "@#{$2.presence || $1}"
            end
          end
          text = text.gsub(/<#(?:[A-Z0-9]+)\|([^>]+)>/) { "##{$1}" }
          text = text.gsub(/<#([A-Z0-9]+)>/) { "##{$1}" }
          # @here, @channel and @everyone stay plain text: Smartfire only
          # turns @[Name] tokens into mentions (see MENTION_TOKEN_PATTERN),
          # so these can never notify anyone.
          text = text.gsub(/<!(here|channel|everyone)>/) { "@#{$1}" }
          text = text.gsub(/<!subteam\^[A-Z0-9]+\|@?([^>]+)>/) { "@#{$1}" }
          text = text.gsub(/<!subteam\^[A-Z0-9]+>/) { "@group" }
          text.gsub(/<!date\^[^\s|>]+(?:\^[^\s|>]+)*(?:\|([^>]*))?>/) { $1.to_s }
        end

        # Converted [text](url) links are shielded while emphasis runs so a
        # * or _ inside a URL or link label is never treated as mrkdwn.
        def convert_emphasis(text)
          shields = {}
          text = text.gsub(/\[[^\]\n]*\]\([^)\n]*\)/) do |link|
            shield_key(shields.size).tap { |key| shields[key] = link }
          end
          text = text.gsub(BOLD_PATTERN) { "**#{$1}**" }
          text = text.gsub(ITALIC_PATTERN) { "*#{$1}*" }
          text = text.gsub(STRIKE_PATTERN) { "~~#{$1}~~" }
          shields.each { |key, link| text = text.gsub(key) { link } }
          text
        end

        def shield_key(index)
          "SMARTFIRESLACKLINK#{index}END"
        end

        def convert_bullets(text)
          text.gsub(/^(\s*)• /, '\1- ')
        end

        def strip_skin_tones(text)
          text.gsub(SKIN_TONE_PATTERN, "")
        end

        def apply_me_message(text, message)
          return text unless message["subtype"] == "me_message" && text.strip.present?

          text.lines.map do |line|
            line.strip.present? ? "*#{line.strip}*" : line
          end.join
        end

        # Files are not imported (a settled user decision); each file
        # becomes a link line back to Slack.
        def append_files(text, message)
          lines = Array(message["files"]).filter_map do |file|
            name = file["name"].presence || file["title"].presence || "file"
            url = file["permalink"].presence || file["permalink_public"].presence || file["url_private"]
            "📎 [#{name}](#{url})" if url.present?
          end
          return [ text, 0 ] if lines.empty?

          [ [ text.strip.presence, *lines ].compact.join("\n\n"), lines.size ]
        end

        def truncate(text)
          return [ text, false ] if text.length <= Message::Markdown::SOURCE_LIMIT

          [ text.truncate(Message::Markdown::SOURCE_LIMIT, omission: TRUNCATION_OMISSION), true ]
        end
    end
  end
end
