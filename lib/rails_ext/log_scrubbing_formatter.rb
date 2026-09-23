# Bot requests carry the bot key as a URL path segment (/rooms/:room_id/:bot_key/...):
# either the long-lived "<id>-<token>" key or a signed webhook reply token (a
# message-verifier blob ending in "--<hex digest>", percent-encoded in the path).
# config.filter_parameters redacts query and form parameters but never path segments,
# so the credential would otherwise be written verbatim to the request log. Redact it
# wherever it appears in a formatted log line. Only credential-shaped segments match:
# literal subpaths such as /rooms/:id/agents/messages pass through untouched.
class LogScrubbingFormatter < ::Logger::Formatter
  BOT_KEY_IN_PATH = %r{(/rooms/\d+/)(?:\d+-[A-Za-z0-9]+|[^/\s"<>]*--[0-9a-f]+)}

  def call(severity, time, progname, message)
    scrub(super)
  end

  private
    def scrub(line)
      line.gsub(BOT_KEY_IN_PATH, '\1[FILTERED]')
    end
end
