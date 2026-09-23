# Parses Slack-style search operators out of a raw query string and applies
# them to message, thread, and event scopes.
#
# Supported operators (each may repeat; from:/in: values combine with OR,
# everything else combines with AND):
#
#   from:@name        messages whose creator's name contains name
#   in:#room          messages in rooms whose name contains room
#   has:link|file|image|pin
#   before:YYYY-MM-DD messages created before the date
#   after:YYYY-MM-DD  messages created after the date
#   on:YYYY-MM-DD     messages created on the date
#   is:thread         messages posted inside a thread
#
# from:/in: values are single tokens (no spaces); a leading @ or # is
# optional. Direct rooms have no name, so in: never matches them. has:image
# only sees uploaded files: Drive attachments store no MIME type. has:link
# is a stored-body heuristic (an anchor or a bare http(s) URL), so legacy
# messages whose links were never stored as anchors may be missed.
#
# Anything left after the valid operators are removed becomes the FTS
# phrase expression, exactly as before (every word-character run quoted as
# a phrase). Operators whose value is missing or unparseable stay plain
# text. Every user-supplied value reaches the database only through bound
# parameters with LIKE wildcards escaped.
class SearchQuery
  OPERATOR_PATTERN = /(?:\A|\s)(?<token>(?<name>from|in|has|before|after|on|is):(?<value>\S+))/
  TRAILING_PUNCTUATION = [ ",", ".", "!", "?", ";", ":", ")" ].freeze
  HAS_VALUES = %w[link file image pin].freeze
  DATE_PATTERN = /\A\d{4}-\d{2}-\d{2}\z/
  SECTION_LIMIT = 10

  Chip = Data.define(:token, :label, :remove_query)

  attr_reader :raw, :from_names, :in_rooms, :has_values,
    :before_date, :after_date, :on_date, :text

  def self.parse(raw)
    new(raw.to_s)
  end

  def initialize(raw)
    @raw = raw.to_s
    @from_names = []
    @in_rooms = []
    @has_values = []
    @chips = []

    remaining = @raw.dup
    @raw.scan(OPERATOR_PATTERN) do |token, name, value|
      cleaned = clean_value(name, value)
      next unless cleaned

      case name
      when "from" then @from_names << cleaned
      when "in" then @in_rooms << cleaned
      when "has" then @has_values |= [ cleaned ]
      when "before" then @before_date = cleaned
      when "after" then @after_date = cleaned
      when "on" then @on_date = cleaned
      when "is" then @thread_only = true
      end

      @chips << Chip.new(token:, label: "#{name}: #{cleaned.is_a?(Date) ? cleaned.iso8601 : cleaned}", remove_query: nil)
      remaining.sub!(token, "")
    end

    @text = remaining.squish
    @chips = @chips.map { |chip| chip.with(remove_query: remove_token(chip.token)) }
  end

  def thread_only?
    !!@thread_only
  end

  def filters?
    from_names.any? || in_rooms.any? || has_values.any? ||
      before_date.present? || after_date.present? || on_date.present? || thread_only?
  end

  def chips
    @chips
  end

  def text_tokens
    @text.scan(/[[:word:]]+/)
  end

  # The FTS5 MATCH expression for the operator-free text. Every token is
  # quoted as a phrase, so operator words (AND, OR, NOT), trailing
  # operators and quote characters are searched literally instead of
  # raising "fts5: syntax error" or silently becoming a boolean query.
  def match_expression
    text_tokens.map { |token| %("#{token}") }.join(" ").presence
  end

  def blank_query?
    text_tokens.empty? && !filters?
  end

  def apply_to_messages(scope)
    scope = scope.where(system_note: false)
    scope = scope.where(creator_id: users_matching(from_names)) if from_names.any?
    scope = scope.where(room_id: rooms_matching(in_rooms)) if in_rooms.any?
    has_values.each { |has| scope = apply_has(scope, has) }
    scope = scope.where("messages.created_at < ?", before_date.in_time_zone.beginning_of_day) if before_date
    scope = scope.where("messages.created_at >= ?", (after_date + 1).in_time_zone.beginning_of_day) if after_date
    scope = scope.where(created_at: on_date.all_day) if on_date
    scope = scope.where.not(thread_id: nil) if thread_only?
    scope = scope.search(match_expression) if match_expression
    scope
  end

  # Board posts, work threads, and events match the operator-free text
  # against their names (events also their description), further narrowed
  # by in: only; the people, attachment, date, and thread operators are
  # message filters. Each section is scoped to rooms the user belongs to.
  def board_posts_for(user)
    return ChannelThread.none if text_tokens.empty?

    scope = ChannelThread.where(room_id: user.rooms.boards.select(:id))
    scope = scope.where(room_id: rooms_matching(in_rooms)) if in_rooms.any?
    with_thread_name_text(scope).ordered.includes(:room).limit(SECTION_LIMIT)
  end

  def work_threads_for(user)
    return ChannelThread.none if text_tokens.empty?

    scope = ChannelThread.work.where(room_id: user.rooms.where.not(type: "Rooms::Board").select(:id))
    scope = scope.where(room_id: rooms_matching(in_rooms)) if in_rooms.any?
    with_thread_name_text(scope).ordered.includes(:room).limit(SECTION_LIMIT)
  end

  def events_for(user)
    return Event.none if text_tokens.empty?

    scope = Event.where(room_id: user.rooms.select(:id))
    scope = scope.where(room_id: rooms_matching(in_rooms)) if in_rooms.any?
    text_tokens.reduce(scope) do |current, token|
      pattern = like_pattern(token)
      current.where("LOWER(events.title) LIKE ? ESCAPE '\\' OR LOWER(events.description) LIKE ? ESCAPE '\\'", pattern, pattern)
    end.ordered.includes(:room).limit(SECTION_LIMIT)
  end

  private
    def clean_value(name, value)
      case name
      when "from", "in"
        strip_trailing_punctuation(value.delete_prefix(name == "from" ? "@" : "#")).presence
      when "has"
        value.downcase.presence_in(HAS_VALUES)
      when "before", "after", "on"
        Date.iso8601(value) if value.match?(DATE_PATTERN)
      when "is"
        true if value.downcase == "thread"
      end
    rescue Date::Error
      nil
    end

    # Trailing punctuation pasted after a mention ("from:@jz,"). A plain
    # loop: the obvious /[,.!?;:)]+\z/ scans every start position and
    # backtracks the plus at each one, which is quadratic on untrusted
    # filter values.
    def strip_trailing_punctuation(value)
      value = value.chop while value.end_with?(*TRAILING_PUNCTUATION)
      value
    end

    def remove_token(token)
      raw.sub(token, "").squish.presence || ""
    end

    def users_matching(names)
      names.reduce(nil) do |relation, name|
        condition = User.where("LOWER(users.name) LIKE ? ESCAPE '\\'", like_pattern(name))
        relation ? relation.or(condition) : condition
      end || User.none
    end

    def rooms_matching(names)
      names.reduce(nil) do |relation, name|
        condition = Room.where("LOWER(rooms.name) LIKE ? ESCAPE '\\'", like_pattern(name))
        relation ? relation.or(condition) : condition
      end || Room.none
    end

    def with_thread_name_text(scope)
      text_tokens.reduce(scope) do |current, token|
        current.where("LOWER(channel_threads.name) LIKE ? ESCAPE '\\'", like_pattern(token))
      end
    end

    def like_pattern(term)
      "%#{ActiveRecord::Base.sanitize_sql_like(term.downcase)}%"
    end

    def apply_has(scope, has)
      case has
      when "link"
        scope.joins(:rich_text_body).where(
          "action_text_rich_texts.body LIKE '%href=%' OR " \
          "action_text_rich_texts.body LIKE '%http://%' OR " \
          "action_text_rich_texts.body LIKE '%https://%'"
        )
      when "file"
        attachments = ActiveStorage::Attachment.where(record_type: "Message", name: "attachment").select(:record_id)
        scope.where(id: attachments).or(scope.where(id: DriveAttachment.select(:message_id)))
      when "image"
        scope.joins(attachment_attachment: :blob).where("active_storage_blobs.content_type LIKE ?", "image/%")
      when "pin"
        scope.where(id: MessagePin.select(:message_id))
      end
    end
end
