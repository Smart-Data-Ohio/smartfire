module SlashCommands
  # The one registry of chat slash commands. Each command carries a name,
  # a description, an argument hint for the picker, a permission check,
  # a handler, and whether picking it inserts "/name " and waits for
  # arguments (true) or runs it immediately (false). The picker, the
  # dispatcher, and the agent-command collision check all read from
  # here, so adding a command means adding one entry plus a handler
  # method.
  #
  # permission receives (user, room, thread) and answers whether the
  # command is offered and runnable there. handler receives a Context
  # and returns a Result. takes_arguments is false for commands that
  # need no argument and for UI-opening commands whose argument is
  # optional: picking them runs at once, and typing arguments still
  # works because the picker closes once an argument starts.
  module Registry
    Context = Data.define(:user, :room, :thread, :args, :routes)
    Result = Data.define(:kind, :message, :url, :notice, :payload) do
      def self.posted(message, notice: nil)
        new(:posted, nil, nil, notice, { message_id: message.id })
      end

      def self.ephemeral(message)
        new(:ephemeral, message, nil, nil, {})
      end

      def self.error(message)
        new(:error, message, nil, nil, {})
      end

      def self.open_url(url)
        new(:open_url, nil, url, nil, {})
      end

      def self.open_poll
        new(:open_poll, nil, nil, nil, {})
      end

      def self.start_huddle(room)
        new(:start_huddle, nil, nil, nil, { room_id: room.id })
      end
    end
    Command = Data.define(:name, :description, :arg_hint, :permission, :handler, :takes_arguments)

    MEMBER = ->(_user, _room, _thread) { true }
    ROOT_ONLY = ->(_user, _room, thread) { thread.nil? }

    COMMANDS = [
      Command.new("huddle", "Start a call in this room", "", MEMBER, :handle_huddle, false),
      Command.new("event", "Open the event form prefilled", "<title> <when>", MEMBER, :handle_event, false),
      Command.new("poll", "Open the poll builder", "", ROOT_ONLY, :handle_poll, false),
      Command.new("remind", "Post and remind yourself about it later", "<when> <text>", MEMBER, :handle_remind, true),
      Command.new("status", "Set your custom status", "<emoji> <text>", MEMBER, :handle_status, true),
      Command.new("dnd", "Toggle Do Not Disturb, optionally for a while", "[duration|off]", MEMBER, :handle_dnd, true),
      Command.new("ooo", "Set out of office with an optional note", "<when> [note]|off", MEMBER, :handle_ooo, true),
      Command.new("shrug", "Post with a shrug", "[text]", MEMBER, :handle_shrug, true),
      Command.new("me", "Post an action line", "<action>", MEMBER, :handle_me, true),
      Command.new("play", "Play a chat sound", "<sound>", MEMBER, :handle_play, true)
    ].freeze

    class << self
      def all
        COMMANDS
      end

      def lookup(name)
        COMMANDS.find { |command| command.name == name.to_s.downcase }
      end

      def builtin?(name)
        lookup(name).present?
      end

      def available_for(user, room, thread: nil)
        COMMANDS.select { |command| command.permission.call(user, room, thread) }
      end
    end
  end
end
