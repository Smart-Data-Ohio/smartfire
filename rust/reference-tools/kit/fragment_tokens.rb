# Which session every authenticity token on a room's pages belongs to, as our Rails renders them
# for two people in turn: David, then Jason, then each again (the second pass reads the fragment
# cache the first one filled). For crates/campfire's room page tests and the WS4 report.
#
#   PARITY_OWNER=... parity/bin/reference runner --seed default reference-tools/kit/fragment_tokens.rb
#
# Each token is checked with RequestForgeryProtection itself (valid_authenticity_token? against
# each person's session, for the form's own action and method, so per-form tokens count too).
require_relative "../support"

class FragmentTokens
  include ReferenceTools

  ROOM = 486777696 # "All Talk"

  def run
    people = { "david" => User.find(127326141), "jason" => User.find(149087659) }.transform_values { |user| sign_in(user) }
    room = Room.find(ROOM)
    older_than = room.root_messages.ordered.last(Message::PAGE_SIZE).first
    pages = { "room" => "/rooms/#{ROOM}" }
    pages["older messages"] = "/rooms/#{ROOM}/messages?before=#{older_than.id}" if older_than
    # A page of messages (cached per message) that includes the room's boosted ones.
    ordered = room.root_messages.ordered.to_a
    boosted = ordered.select { |message| message.boosts.any? }
    if (first_boosted = boosted.first) && (before = ordered[ordered.index(first_boosted) - 1])
      pages["messages with boosts"] = "/rooms/#{ROOM}/messages?after=#{before.id}"
    end
    boosted.each { |message| pages["room at boosted #{message.id}"] = "/rooms/#{ROOM}/@#{message.id}" }

    result = { cache_store: Rails.cache.class.name, pages: {} }
    pages.each do |label, path|
      renders = [ "david", "jason", "david", "jason" ].map { |name| [ name, render(people.fetch(name), path) ] }
      result[:pages][label] = renders.each_with_index.map do |(name, page), pass|
        owners = page[:tokens].map { |token| owner(people, token) }
        { pass: pass / 2 + 1, viewer: name, status: page[:status], forms: page[:tokens].count { |t| t[:kind] == "form" },
          tokens_by_owner: owners.tally, other_session_tokens: owners.count { |o| o != name && o != "none" },
          forms_by_action: page[:tokens].filter_map { |t| "#{t[:method]} #{t[:action]}" if t[:kind] == "form" }.tally,
          unverified: page[:tokens].zip(owners).filter_map { |token, o| "#{token[:kind]} #{token[:method]} #{token[:action]}" if o == "none" }.tally }
      end
      david, jason = renders[2][1], renders[3][1]
      result[:pages][label] << { shared_token_values: (david[:tokens].map { _1[:value] } & jason[:tokens].map { _1[:value] }).size }
    end

    # What a broadcast (Turbo's `ApplicationController.render`, outside any request) puts in the
    # same forms: the reactions a boost broadcasts, and a boosted message as it's appended.
    if (message = boosted.first)
      rendered = Rails.application.executor.wrap do
        { "messages/boosts/reactions" => ApplicationController.render(formats: [ :html ], partial: "messages/boosts/reactions", locals: { message: message }),
          "messages/message" => ApplicationController.render(formats: [ :html ], partial: "messages/message", locals: { message: message }) }
      end
      result[:broadcasts] = rendered.transform_values do |html|
        found = tokens(html, "/")
        { forms: html.scan(/<form\b/).size, tokens: found.size,
          owners: found.map { |token| owner(people, token) }.tally,
          token_inputs: html.scan(/<input[^>]*name="authenticity_token"[^>]*>/).first(1) }
      end
    end
    puts JSON.pretty_generate(result)
  end

  private
    def sign_in(user)
      session = user.sessions.start!(user_agent: USER_AGENT, ip_address: "127.0.0.1", two_factor_verified: true)
      jar = request_for.cookie_jar
      jar.signed.permanent[:session_token] = { value: session.token, httponly: true, same_site: :lax }
      { cookies: { "session_token" => jar[:session_token] } }
    end

    # GET the page (twice the first time, so the session cookie that holds the CSRF token exists).
    def render(person, path)
      unless person[:cookies]["_campfire_session"]
        _, headers, = perform(:get, path, cookies: person[:cookies])
        person[:cookies]["_campfire_session"] = set_cookies(headers).dig("_campfire_session", "raw")
        person[:csrf] = read_cookie(:encrypted, "_campfire_session", person[:cookies]["_campfire_session"])["_csrf_token"]
      end
      status, _, body = perform(:get, path, cookies: person[:cookies])
      { status: status, tokens: tokens(body, URI(path).path) }
    end

    def tokens(html, page_path)
      meta = html.scan(/<meta name="csrf-token" content="([^"]*)"/).map { |(value)| { kind: "meta", value: value, action: "/", method: "POST" } }
      forms = html.scan(%r{<form\b([^>]*)>(.*?)</form>}m).filter_map do |attributes, inner|
        value = inner[/name="authenticity_token" value="([^"]*)"/, 1] or next
        action = CGI.unescapeHTML(attributes[/\saction="([^"]*)"/, 1].to_s)
        method = (inner[/name="_method" value="([^"]*)"/, 1] || attributes[/\smethod="([^"]*)"/, 1] || "post").upcase
        # A relative action (`url: "#"`) gets the per-form token of the page's own path
        # (RequestForgeryProtection#normalize_relative_action_path).
        path = action.start_with?("/") ? URI(action).path : page_path
        { kind: "form", value: value, action: path, method: method }
      end
      meta + forms
    end

    def owner(people, token)
      people.find { |_, person| valid?(person[:csrf], token) }&.first || "none"
    end

    def valid?(csrf, token)
      controller = csrf_controller(csrf, path: token[:action], method: token[:method])
      controller.send(:valid_authenticity_token?, controller.session, token[:value])
    end
end

FragmentTokens.new.run
