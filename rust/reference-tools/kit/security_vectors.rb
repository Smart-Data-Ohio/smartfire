# Generates vectors/kit_security.json: the security surface crates/kit and the campfire
# before-action chain reproduce (CSRF, security headers and CSP, rate limiting, session keys,
# parameter filtering and log scrubbing), taken from the reference app itself.
#
# Needs Redis (sessions#create rate-limits through the Redis cache store), so it runs through the
# parity runner, once per SSL mode:
#
#   parity/bin/reference runner reference-tools/kit/security_vectors.rb plain
#   parity/bin/reference runner -e DISABLE_SSL= -e LIVEKIT_URL=wss://livekit.campfire.test:7880 \
#     reference-tools/kit/security_vectors.rb ssl
#
# Each run replaces its own sections of the file and keeps the others.
require_relative "../support"
require "action_controller/test_case"

class KitSecurityVectors
  include ReferenceTools

  PASSWORD = "secret123456"

  def initialize(mode)
    @mode = mode
  end

  def generate
    travel_to(NOW)
    begin
      sections = { "headers_#{@mode}" => header_vectors }
      if @mode == "plain"
        sections.merge!(
          "csrf" => csrf_vectors,
          "sign_in" => sign_in_vectors,
          "rate_limit" => rate_limit_vectors,
          "session_keys" => session_key_vectors,
          "csp" => csp_vectors,
          "parameter_filter" => parameter_filter_vectors,
          "log_scrubbing" => log_scrubbing_vectors,
          "admin_idle_timeout" => admin_idle_timeout_vectors
        )
      end
      sections
    ensure
      travel_back
    end
  end

  private
    # --- Headers per route class ---------------------------------------------------------------

    # Every response class the port serves, on an empty database (first run) and then with users:
    # rendered pages, redirects, framework endpoints, JSON, the service worker, digested assets,
    # public files, error pages and the ActionController::API report endpoint.
    def header_vectors
      empty_database!
      asset = ActionController::Base.helpers.asset_path("base.css")
      requests = [
        [ "html_page", :get, "/first_run", {} ],
        [ "redirect", :get, "/session/new", {} ],
        [ "health", :get, "/up", {} ],
        [ "json", :get, "/webmanifest.json", { "HTTP_ACCEPT" => "application/json" } ],
        [ "javascript", :get, "/service-worker.js", { "HTTP_ACCEPT" => "*/*" } ],
        [ "asset", :get, asset, { "HTTP_ACCEPT" => "text/css" } ],
        [ "asset_head", :head, asset, { "HTTP_ACCEPT" => "text/css" } ],
        [ "missing_asset", :get, "/assets/missing-0000000000000000.css", {} ],
        [ "public_file", :get, "/robots.txt", {} ],
        [ "not_found", :get, "/no/such/page", {} ],
        [ "csp_report", :post, "/csp_reports", csp_report_env(CSP_REPORT) ]
      ]
      empty = requests.to_h { |name, method, path, env| [ name, response_vector(method, path, env) ] }

      reset_database!
      page = perform(:get, "/session/new")
      session_raw = set_cookies(page[1]).dig("_campfire_session", "raw")
      seeded = {
        "sign_in_page" => response_vector(:get, "/session/new", {}),
        "sign_in_page_with_session" => response_vector(:get, "/session/new", {}, cookies: { "_campfire_session" => session_raw }),
        "json_head" => response_vector(:get, "/session/new", { "HTTP_ACCEPT" => "application/json" }),
        "forgery" => response_vector(:post, "/session", {}, params: { email_address: "david@example.com", password: "wrong" }),
        "unauthorized_render" => response_vector(:post, "/session", { "REMOTE_ADDR" => "10.9.9.9" },
          cookies: { "_campfire_session" => session_raw },
          params: { email_address: "david@example.com", password: "wrong", authenticity_token: meta_token(page[2]) })
      }

      { "asset_path" => asset, "requests" => empty.merge(seeded) }
    end

    def response_vector(method, path, env, cookies: {}, params: nil)
      status, headers, body = perform(method, path, cookies: cookies, params: params, headers: env)
      {
        "method" => method.to_s.upcase, "path" => path, "env" => env,
        "cookies" => cookies.keys, "status" => status,
        "headers" => headers.to_h.transform_keys(&:downcase).transform_values { |v| v.is_a?(Array) ? v.join("\n") : v.to_s },
        "body_prefix" => body.b[0, 80].force_encoding("UTF-8").scrub
      }
    end

    CSP_REPORT = {
      "csp-report" => {
        "document-uri" => "https://campfire.test/rooms/1?secret=1",
        "violated-directive" => "script-src-elem",
        "effective-directive" => "script-src-elem",
        "blocked-uri" => "https://evil.test:8443/x.js?q=1",
        "source-file" => "https://campfire.test/assets/application-abc.js",
        "line-number" => 12
      }
    }.freeze

    def csp_report_env(report, type: "application/csp-report")
      body = report.is_a?(String) ? report : JSON.generate(report)
      { "CONTENT_TYPE" => type, :input => body, "REMOTE_ADDR" => "10.7.7.7" }
    end

    def empty_database!
      ActiveRecord::Schema.verbose = false
      silence_stream($stdout) { load Rails.root.join("db/schema.rb") }
    end

    def meta_token(body)
      body[/<meta name="csrf-token" content="([^"]+)"/, 1]
    end

    def form_token(body, action)
      body[%r{<form[^>]*action="#{Regexp.escape(action)}"[^>]*>.*?name="authenticity_token" value="([^"]+)"}m, 1]
    end

    # --- Sign-in page and CSRF, end to end ------------------------------------------------------

    def sign_in_vectors
      reset_database!
      status, headers, body = perform(:get, "/session/new")
      cookie = set_cookies(headers).fetch("_campfire_session")
      session = read_cookie(:encrypted, "_campfire_session", cookie["raw"])
      meta = meta_token(body)
      form = form_token(body, "http://#{HOST}/session")

      # A second visitor: its tokens must not work with the first visitor's session.
      _, other_headers, other_body = perform(:get, "/session/new")
      other_cookie = set_cookies(other_headers).fetch("_campfire_session")

      page = csrf_controller(session["_csrf_token"], path: "/session/new", method: "GET")
      token_for_other_form = page.send(:form_authenticity_token, form_options: { action: "/first_run", method: "post" })
      real_token = session["_csrf_token"]
      global_unmasked = Base64.urlsafe_encode64(page.send(:global_csrf_token), padding: false)

      cookies = { "_campfire_session" => cookie["raw"] }
      ip = 0
      post = lambda do |label, token: nil, header: nil, origin: nil, with_cookies: cookies, path: "/session", method: "POST"|
        ip += 1
        env = { "REMOTE_ADDR" => "10.1.0.#{ip}" }
        env["HTTP_X_CSRF_TOKEN"] = header if header
        env["HTTP_ORIGIN"] = origin if origin
        params = { email_address: "david@example.com", password: "wrong" }
        params[:authenticity_token] = token if token
        params[:_method] = method.downcase if method != "POST"
        status, response_headers, _ = perform(:post, path, cookies: with_cookies, params: params, headers: env)
        { "case" => label, "token" => token, "header" => header, "origin" => origin, "cookies" => with_cookies.keys,
          "path" => path, "status" => status, "set_cookie" => set_cookies(response_headers).keys }
      end

      posts = [
        post.("form token", token: form),
        post.("meta token as header", header: meta),
        post.("meta token as param", token: meta),
        post.("unmasked session token", token: real_token),
        post.("unmasked global token", token: global_unmasked),
        post.("per-form token for another form", token: token_for_other_form),
        post.("no token"),
        post.("forged token", token: Base64.urlsafe_encode64(SecureRandom.random_bytes(64), padding: false)),
        post.("token from another session", token: form_token(other_body, "http://#{HOST}/session")),
        post.("meta token from another session as header", header: meta_token(other_body)),
        post.("form token without the session cookie", token: form, with_cookies: {}),
        post.("form token with the other session's cookie", token: form, with_cookies: { "_campfire_session" => other_cookie["raw"] }),
        post.("form token, same origin", token: form, origin: "http://#{HOST}"),
        post.("form token, cross origin", token: form, origin: "http://evil.test"),
        post.("form token, null origin", token: form, origin: "null"),
        post.("bad header, good param", token: form, header: "bad"),
        post.("good header, bad param", token: "bad", header: meta),
        post.("form token, tampered", token: tamper_first(form))
      ]

      # A successful sign-in from the page's form: what start_new_session_for writes.
      _, login_headers, _ = perform(:post, "/session", cookies: cookies,
        params: { email_address: "jason@example.com", password: PASSWORD, authenticity_token: form },
        headers: { "REMOTE_ADDR" => "10.2.0.1" })
      login_cookies = set_cookies(login_headers)

      {
        "status" => status,
        "session_cookie_raw" => cookie["raw"],
        "set_cookie" => cookie["header"],
        "session" => session,
        "csrf_meta_token" => meta,
        "session_form_token" => form,
        "other_session_cookie_raw" => other_cookie["raw"],
        "csp_nonce" => body[/<meta name="csp-nonce" content="([^"]+)"/, 1],
        "head" => body[%r{<head>.*?</head>}m],
        "posts" => posts,
        "sign_in" => {
          "set_cookie_names" => login_cookies.keys,
          "location" => login_headers["location"],
          "device_id_set_cookie" => login_cookies.dig("device_id", "header"),
          "device_id" => login_cookies["device_id"] && read_cookie(:signed, "device_id", login_cookies.dig("device_id", "raw")),
          "session" => read_cookie(:encrypted, "_campfire_session", login_cookies.dig("_campfire_session", "raw")),
          "session_row" => Session.last.attributes.slice("device_id", "two_factor_verified_at").transform_values { |v| v.respond_to?(:iso8601) ? iso(v) : v }
        }
      }
    end

    def tamper_first(string)
      (string[0] == "A" ? "B" : "A") + string[1..]
    end

    # --- The token algorithm, case by case --------------------------------------------------

    CSRF_SESSION_TOKEN = "qK3zv7cQ2oYlP4-sX6JtW0nBf9eR1uHaMdLgE5iVy8k"
    OTHER_SESSION_TOKEN = "Zm9vYmFyYmF6cXV4Zm9vYmFyYmF6cXV4Zm9vYmFyYmE"

    def csrf_vectors
      controller = csrf_controller(CSRF_SESSION_TOKEN)
      global_tokens = 2.times.map { controller.send(:form_authenticity_token) }
      form_targets = [
        [ "/session", "post", "/session/new" ],
        [ "http://#{HOST}/session", "post", "/session/new" ],
        [ "/rooms/1/", "patch", "/rooms/1/edit" ],
        [ "/rooms/1?x=1", "delete", "/rooms/1" ],
        [ "messages", "post", "/rooms/1" ],
        [ "./involvement", "put", "/rooms/1/" ],
        [ "", "post", "/account/edit" ],
        [ "/rooms/1/messages#anchor", "post", "/rooms/1" ]
      ]
      form_tokens = form_targets.map do |action, method, page_path|
        page = csrf_controller(CSRF_SESSION_TOKEN, path: page_path, method: "GET")
        normalized = page.send(:normalize_action_path, action)
        { "action" => action, "method" => method, "page_path" => page_path, "normalized_action_path" => normalized,
          "unmasked_hex" => page.send(:per_form_csrf_token, nil, normalized, method).unpack1("H*"),
          "token" => page.send(:form_authenticity_token, form_options: { action: action, method: method }) }
      end

      raw_real = Base64.urlsafe_decode64(CSRF_SESSION_TOKEN)
      global_hmac = OpenSSL::HMAC.digest("SHA256", raw_real, "!real_csrf_token")
      pad = "\x01".b * 32
      submitted = global_tokens.map { |t| [ "masked global token", t ] } + [
        [ "unmasked session token", CSRF_SESSION_TOKEN ],
        [ "session token masked with a pad", Base64.urlsafe_encode64(pad + xor(pad, raw_real), padding: false) ],
        [ "unmasked global token", Base64.urlsafe_encode64(global_hmac, padding: false) ],
        [ "masked global token, padded", Base64.urlsafe_encode64(Base64.urlsafe_decode64(global_tokens.first)) ],
        [ "masked global token, standard alphabet", global_tokens.first.tr("-_", "+/") ],
        [ "token for another session", csrf_controller(OTHER_SESSION_TOKEN).send(:form_authenticity_token) ],
        [ "tampered masked token", tamper_first(global_tokens.first) ],
        [ "truncated masked token", global_tokens.first[0..-5] ],
        [ "invalid base64", "!!!!" ],
        [ "empty", "" ]
      ] + form_tokens.map { |f| [ "per-form token #{f["method"]} #{f["action"]}", f["token"] ] }

      requests = [
        [ "/session", "POST" ], [ "/session/", "POST" ], [ "/session", "PUT" ], [ "/rooms/1", "PATCH" ],
        [ "/rooms/1", "DELETE" ], [ "/rooms/1/messages", "POST" ], [ "/rooms/1//involvement", "PUT" ], [ "/account/edit", "POST" ], [ "/other", "POST" ]
      ]
      validity = submitted.flat_map do |label, token|
        requests.map do |path, method|
          c = csrf_controller(CSRF_SESSION_TOKEN, path: path, method: method)
          { "case" => label, "token" => token, "path" => path, "method" => method, "expected" => c.send(:valid_authenticity_token?, c.session, token) }
        end
      end

      origins = [
        [ nil, "http://#{HOST}" ], [ "http://#{HOST}", "http://#{HOST}" ], [ "https://#{HOST}", "http://#{HOST}" ],
        [ "http://evil.test", "http://#{HOST}" ], [ "null", "http://#{HOST}" ], [ "http://#{HOST}:8080", "http://#{HOST}" ],
        [ "http://#{HOST}:8080", "http://#{HOST}:8080" ], [ "HTTP://#{HOST.upcase}", "http://#{HOST}" ], [ "https://#{HOST}", "https://#{HOST}" ],
        [ "", "http://#{HOST}" ]
      ].map do |origin, base_url|
        uri = URI(base_url)
        env = { "HTTP_HOST" => uri.port == uri.default_port ? uri.host : "#{uri.host}:#{uri.port}", "rack.url_scheme" => uri.scheme }
        env["HTTPS"] = "on" if uri.scheme == "https"
        env["HTTP_ORIGIN"] = origin if origin
        request = request_for(env: env)
        controller = ApplicationController.new.tap { |c| c.set_request!(request) }
        expected = begin
          controller.send(:valid_request_origin?)
        rescue ActionController::InvalidAuthenticityToken
          "raises"
        end
        { "origin" => origin, "base_url" => request.base_url, "expected" => expected }
      end

      {
        "session_token" => CSRF_SESSION_TOKEN,
        "global_token_hex" => global_hmac.unpack1("H*"),
        "global_tokens" => global_tokens,
        "form_tokens" => form_tokens,
        "validity" => validity,
        "origin" => origins,
        "per_form_csrf_tokens" => ApplicationController.per_form_csrf_tokens,
        "forgery_protection_origin_check" => ApplicationController.forgery_protection_origin_check,
        "csrf_meta_tags" => csrf_meta_tags_example
      }
    end

    def csrf_meta_tags_example
      page = csrf_controller(CSRF_SESSION_TOKEN, path: "/", method: "GET")
      view = page.view_context
      { "html" => view.csrf_meta_tags, "token_tag" => view.send(:token_tag, nil, form_options: { action: "/session", method: "post" }) }
    end

    def xor(a, b)
      a.bytes.zip(b.bytes).map { |x, y| x ^ y }.pack("C*")
    end

    # --- Rate limiting -----------------------------------------------------------------------

    # sessions#create: 10 per 3 minutes per IP in the shared cache, then the 429 rejection render.
    def rate_limit_vectors
      reset_database!
      _, headers, body = perform(:get, "/session/new")
      cookies = { "_campfire_session" => set_cookies(headers).dig("_campfire_session", "raw") }
      token = meta_token(body)
      env = { "REMOTE_ADDR" => "10.3.0.1" }
      params = { email_address: "david@example.com", password: "wrong", authenticity_token: token }
      responses = 12.times.map { perform(:post, "/session", cookies: cookies, params: params, headers: env) }
      other_ip = perform(:post, "/session", cookies: cookies, params: params, headers: { "REMOTE_ADDR" => "10.3.0.2" })
      # A forged request fails verification before the limiter counts it.
      forged = perform(:post, "/session", cookies: cookies, params: params.merge(authenticity_token: "bad"), headers: { "REMOTE_ADDR" => "10.3.0.3" })
      limited = responses.last
      {
        "statuses" => responses.map(&:first),
        "other_ip_status" => other_ip.first,
        "forged_status" => forged.first,
        "limited" => { "status" => limited[0], "content_type" => limited[1]["content-type"],
                       "alert" => limited[2][/<div[^>]*class="[^"]*flash[^"]*"[^>]*>.*?<\/div>/m] || limited[2][/Too many requests or unauthorized\./],
                       "has_sign_in_form" => limited[2].include?(%(action="http://#{HOST}/session")) },
        "rejected" => { "status" => responses.first[0] },
        "cache_key" => ["rate-limit", "sessions", nil, "10.3.0.1"].compact.join(":"),
        "csp_reports" => csp_report_rate_limit
      }
    end

    def csp_report_rate_limit
      ContentSecurityPolicyReportsController::RATE_LIMIT_STORE.clear
      statuses = 22.times.map { perform(:post, "/csp_reports", headers: csp_report_env(CSP_REPORT).merge("REMOTE_ADDR" => "10.4.0.1")).first }
      # Nothing asks for params, so a malformed JSON body is never parsed: it's no report, and
      # it counts toward the limit like any other.
      malformed = 21.times.map { perform(:post, "/csp_reports", headers: csp_report_env("{", type: "application/json").merge("REMOTE_ADDR" => "10.4.0.3")).first }
      # The controller reads at most MAX_BODY + 1 bytes of a body of any size.
      oversized = [ "application/csp-report", "application/json" ].to_h do |type|
        body = JSON.generate(CSP_REPORT).ljust(17.megabytes)
        [ type, perform(:post, "/csp_reports", headers: csp_report_env(body, type: type).merge("REMOTE_ADDR" => "10.4.0.4")).first ]
      end
      { "statuses" => statuses, "limit" => ContentSecurityPolicyReportsController::RATE_LIMIT,
        "max_body" => ContentSecurityPolicyReportsController::MAX_BODY,
        "malformed_json_statuses" => malformed, "seventeen_mib_statuses" => oversized }
    end

    # --- Session keys ------------------------------------------------------------------------

    # The keys the authentication, two-step, sudo and reauthentication concerns keep in the cookie
    # session, as the concerns write them, in a cookie Rails issued.
    def session_key_vectors
      reset_database!
      data = {
        "session_id" => "0f1e2d3c4b5a69788796a5b4c3d2e1f0",
        "_csrf_token" => CSRF_SESSION_TOKEN,
        "two_factor_pending_user_id" => @jason.id,
        "two_factor_pending_expires_at" => 10.minutes.from_now.to_i,
        "two_factor_pending_method" => "password",
        "sudo_verified_at" => Time.current.to_i,
        "sudo_pending_request" => { "method" => "PATCH", "path" => "/account?x=1", "params" => { "account" => { "name" => "New" } }, "origin" => "/account/edit" },
        "two_factor_reauthenticated_at" => Time.current.to_i,
        "return_to_after_authenticating" => "http://#{HOST}/rooms/1?a=1&b=<2>"
      }
      raw, header = write_cookie { |jar| jar.encrypted[:_campfire_session] = { value: data, expires: 20.years.from_now } }

      # What each concern concludes from those values, at NOW and later.
      checks = [ NOW, NOW + 9.minutes, NOW + 11.minutes, NOW + 16.minutes ].map do |time|
        at(time) do
          controller = session_controller(data)
          { "now" => iso(time),
            "two_factor_pending_user_id" => controller.send(:two_factor_pending_user)&.id,
            "sudo_verified" => controller.send(:sudo_verified?),
            "reauthenticated" => session_controller(data).send(:consume_google_reauthentication!) }
        end
      end
      {
        "data" => data, "raw" => raw, "set_cookie" => header, "checks" => checks,
        "sudo_timeout_seconds" => SudoMode::SUDO_TIMEOUT.to_i,
        "two_factor_pending_ttl_seconds" => Authentication::TWO_FACTOR_PENDING_TTL.to_i,
        "reauth_ttl_seconds" => TwoFactorReauthentication::REAUTH_TTL.to_i
      }
    end

    def session_controller(data)
      request = request_for(path: "/", env: { "REQUEST_METHOD" => "GET" })
      request.session = ActionController::TestSession.new(data.deep_dup)
      klass = Class.new(ApplicationController) { include SudoMode, TwoFactorReauthentication }
      klass.new.tap { |controller| controller.set_request!(request) }
    end

    # --- CSP ---------------------------------------------------------------------------------

    def csp_vectors
      policy = Rails.application.config.content_security_policy
      livekit = [ nil, "", "wss://livekit.campfire.test", "wss://livekit.campfire.test:7880", "ws://10.0.0.5:7880",
        "https://lk.example.com:443", "http://lk.example.com:80", "ftp://lk.example.com", "livekit.campfire.test",
        "wss://", "not a url", "WSS://LK.Example.com", "wss://user:pw@lk.example.com:7881/path?q=1", "wss://[::1]:7880" ]
      policies = livekit.map do |url|
        with_env("LIVEKIT_URL", url) do
          { "livekit_url" => url, "sources" => ContentSecurityPolicySources.livekit,
            "header" => policy.build(ApplicationController.new, "NONCE", Rails.application.config.content_security_policy_nonce_directives),
            "without_nonce" => policy.build(ApplicationController.new, nil, []) }
        end
      end
      nonces = [ "0f1e2d3c4b5a69788796a5b4c3d2e1f0", "36166e44ee4e3a234bcf50a9f864d2a7" ].map do |sid|
        raw, _ = write_cookie { |jar| jar.encrypted[:_campfire_session] = { value: { "session_id" => sid }, expires: 20.years.from_now } }
        { "session_id" => sid, "nonce" => nonce_for_cookie(raw) }
      end
      without = nonce_for_cookie(nil)
      {
        "policies" => policies,
        "nonces" => nonces,
        "random_nonce_example" => without,
        "nonce_directives" => Rails.application.config.content_security_policy_nonce_directives,
        "report_only" => Rails.application.config.content_security_policy_report_only,
        "default_headers" => Rails.application.config.action_dispatch.default_headers,
        "ssl_options" => Rails.application.config.ssl_options.to_s,
        "immutable_cache_control" => RailsExt::ImmutableAssetHeaders::IMMUTABLE_CACHE_CONTROL,
        "public_file_server_headers" => Rails.application.config.public_file_server.headers
      }
    end

    # `request.content_security_policy_nonce` for a request carrying the session cookie `raw`
    # (unloaded, as the CSP middleware sees it when nothing read the session).
    def nonce_for_cookie(raw)
      request = request_for(cookies: raw ? { "_campfire_session" => raw } : {})
      options = Rails.application.config.session_options
      store = ActionDispatch::Session::CookieStore.new(Rails.application, options)
      ActionDispatch::Request::Session.create(store, request, options)
      request.content_security_policy_nonce
    end

    def with_env(name, value)
      old = ENV[name]
      value.nil? ? ENV.delete(name) : ENV[name] = value
      yield
    ensure
      old.nil? ? ENV.delete(name) : ENV[name] = old
    end

    # --- Parameter filtering and log scrubbing ---------------------------------------------------

    def parameter_filter_vectors
      filters = Rails.application.config.filter_parameters
      filter = ActiveSupport::ParameterFilter.new(filters)
      params = [
        { "email_address" => "david@example.com", "password" => "secret", "commit" => "Sign in" },
        { "user" => { "name" => "David", "email_address" => "d@example.com", "password_confirmation" => "x" }, "authenticity_token" => "tok" },
        { "message" => { "body" => "hello", "client_message_id" => "abc" }, "room_id" => "1" },
        { "message" => { "body_html" => "<b>", "attachment" => "f" }, "body" => "top-level body" },
        { "code" => "4/0Ab", "state" => "xyz", "reauth" => "123456", "otp_attempt" => "1", "totp_code" => "2" },
        { "bot_key" => "1-abc", "api_key" => "k", "webhook" => { "url" => "https://x", "secret" => "s" }, "keys" => { "p256dh" => "a", "auth" => "b" } },
        { "push_subscription_endpoint" => "https://fcm", "endpoint" => "e", "items" => [ { "token" => "a" }, "plain", [ "x" ] ] },
        { "agent" => { "name" => "A", "token_digest" => "d" }, "ssn" => "1", "cvv" => "2", "cvc" => "3", "salt" => "4", "certificate" => "5", "encrypted_thing" => "6" },
        { "two_factor_credential" => { "secret" => "s", "label" => "l" }, "google_account" => { "access_token" => "t", "refresh_token" => "r", "email" => "e" } },
        { "slack_connection" => { "access_token" => "t" }, "q" => "search", "page" => "2", "nested" => { "deep" => { "password" => "p", "ok" => "fine" } } },
        { "Password" => "caps", "PASSWORD_CONFIRMATION" => "caps2", "eMail" => "mixed" }
      ]
      paths = [
        "/session", "/rooms/1?token=abc&page=2", "/search?q=hi&email=a@b.c", "/x?password=1;code=2&ok=3",
        "/rooms/1/messages?message[body]=secret&x=1", "/a?=novalue&flag&api_key=k", "/b?%70assword=encoded"
      ]
      {
        "filter_parameters" => filters.map { |f| f.is_a?(Regexp) ? { "regexp" => f.source } : f.to_s },
        "params" => params.map { |p| { "params" => p, "filtered" => filter.filter(p) } },
        "paths" => paths.map { |path| { "path" => path, "filtered" => request_for(path: path).filtered_path } }
      }
    end

    def log_scrubbing_vectors
      formatter = LogScrubbingFormatter.new
      lines = [
        %(Started POST "/rooms/1/12-AbCdEf123/messages" for 127.0.0.1),
        %(Started POST "/rooms/42/eyJfcmFpbHMiOnsiZGF0YSI6eyJib3RfaWQiOjF9fX0%3D--0123abcdef/messages" for ::1),
        %(Started GET "/rooms/1/agents/messages" for 127.0.0.1),
        %(Started POST "/rooms/7/messages" for 127.0.0.1),
        %(Redirected to http://campfire.test/rooms/3/5-xyz/messages and /rooms/3/9-abc),
        %(Started POST "/rooms/1/foo--bar/messages"),
        %(Started POST "/rooms/1/abc--0f/messages" then "/rooms/2/2-B/x"),
        %(Started GET "/rooms/x/1-abc"),
        %(Parameters: {"bot_key"=>"[FILTERED]"} /rooms/12/1-a)
      ]
      {
        "pattern" => LogScrubbingFormatter::BOT_KEY_IN_PATH.source,
        "lines" => lines.map { |line| { "line" => line, "scrubbed" => formatter.send(:scrub, line) } }
      }
    end

    # --- Admin idle timeout ----------------------------------------------------------------------

    def admin_idle_timeout_vectors
      values = [ nil, "", "7", "1", "0", "-3", "30", "abc", "3.5", " 5", "10days", "0x10", "1_0" ]
      values.map do |value|
        with_env("ADMIN_SESSION_IDLE_TIMEOUT_DAYS", value) { load Rails.root.join("config/initializers/session_lifetimes.rb") }
        { "env" => value, "seconds" => Rails.configuration.x.admin_session_idle_timeout.to_i }
      end
    end
end

mode = ARGV.first || "plain"
path = File.join(ENV["VECTORS_DIR"] || File.join(ENV.fetch("PARITY_WORK"), "vectors"), "kit_security.json")
existing = File.exist?(path) ? JSON.parse(File.read(path)) : {}
sections = KitSecurityVectors.new(mode).generate
merged = existing.merge(sections).merge("secret_key_base" => Rails.application.secret_key_base, "now" => ReferenceTools::NOW.utc.iso8601(3), "host" => ReferenceTools::HOST)
File.write(path, JSON.pretty_generate(merged.sort.to_h) + "\n")
warn "wrote #{sections.keys.join(", ")} to #{path}"
