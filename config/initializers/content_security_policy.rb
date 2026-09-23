# Be sure to restart your server when you modify this file.

# Application-wide Content Security Policy, in report-only mode: browsers
# report violations to /csp_reports (ContentSecurityPolicyReportsController
# logs them, rate-limited) without blocking anything. Switch
# content_security_policy_report_only to false once the reports are quiet.
#
# Sources, and why each is allowed:
# - script-src 'self' plus a per-session nonce (importmap tags and the few
#   inline scripts carry it), 'wasm-unsafe-eval' for the huddle noise
#   suppressor's RNNoise WebAssembly, and Google's Identity Services and
#   API loaders for the Drive picker (accounts.google.com/gsi/,
#   apis.google.com). No 'unsafe-inline' and no 'unsafe-eval'.
# - connect-src 'self' (requests and Action Cable), the LiveKit gateway from
#   LIVEKIT_URL in both its WebSocket and HTTPS forms (the client validates
#   over HTTPS on the same host), and the Google endpoints the picker and
#   Drive sharing call from the browser.
# - img-src 'self', data:, blob: (upload previews), and https:. Link
#   preview images load through the same-origin /embeds/image proxy, so
#   https: remains only for X media and avatars (pbs.twimg.com), GitHub
#   avatars (avatars.githubusercontent.com), and Google avatars.
# - style-src 'self' 'unsafe-inline': views use inline style attributes and
#   the account's custom styles; Google Identity Services adds its own
#   stylesheet. Inline styles cannot run script.
# - frame-src for the Google Picker and sign-in iframes, plus LinkedIn's
#   official embed player (linkedin.com/embed/…), which a LinkedIn post
#   card loads only after the reader clicks "Show embedded post".
# - form-action 'self' plus accounts.google.com, where the Google sign-in and
#   Calendar/Drive connect forms redirect.
# - object-src 'none', base-uri 'self'.
module ContentSecurityPolicySources
  GOOGLE_SCRIPTS = %w[ https://accounts.google.com/gsi/ https://apis.google.com ].freeze
  GOOGLE_CONNECT = %w[ https://accounts.google.com https://www.googleapis.com https://content.googleapis.com ].freeze
  GOOGLE_FRAMES = %w[ https://docs.google.com https://drive.google.com https://accounts.google.com ].freeze
  LINKEDIN_FRAMES = %w[ https://www.linkedin.com ].freeze
  GOOGLE_STYLES = %w[ https://accounts.google.com/gsi/style ].freeze
  GOOGLE_FORMS = %w[ https://accounts.google.com ].freeze

  SCHEME_PAIRS = { "wss" => "https", "ws" => "http", "https" => "wss", "http" => "ws" }.freeze

  # The LiveKit origin in the WebSocket and HTTP forms, read per request so
  # a changed LIVEKIT_URL applies without a restart. Empty when unset or
  # unparseable.
  def self.livekit
    uri = URI.parse(ENV["LIVEKIT_URL"].to_s)
    return [] unless uri.host.present? && SCHEME_PAIRS.key?(uri.scheme)

    port = ":#{uri.port}" if uri.port && uri.port != uri.default_port
    [ uri.scheme, SCHEME_PAIRS.fetch(uri.scheme) ].map { |scheme| "#{scheme}://#{uri.host}#{port}" }
  rescue URI::InvalidURIError
    []
  end
end

Rails.application.configure do
  config.content_security_policy do |policy|
    policy.default_src  :self
    policy.base_uri     :self
    policy.object_src   :none
    policy.script_src   :self, :wasm_unsafe_eval, *ContentSecurityPolicySources::GOOGLE_SCRIPTS
    policy.style_src    :self, :unsafe_inline, *ContentSecurityPolicySources::GOOGLE_STYLES
    policy.img_src      :self, :data, :blob, :https
    policy.font_src     :self, :data
    policy.media_src    :self, :data, :blob
    policy.connect_src  :self, *ContentSecurityPolicySources::GOOGLE_CONNECT, -> { ContentSecurityPolicySources.livekit }
    policy.frame_src    :self, *ContentSecurityPolicySources::GOOGLE_FRAMES, *ContentSecurityPolicySources::LINKEDIN_FRAMES
    policy.worker_src   :self, :blob
    policy.manifest_src :self
    policy.form_action  :self, *ContentSecurityPolicySources::GOOGLE_FORMS
    policy.report_uri   "/csp_reports"
  end

  # One nonce per session rather than per request: Turbo Drive swaps pages
  # without reloading the document, so the browser keeps enforcing the policy
  # (and nonce) from the first full page load, while Turbo gives each inline
  # script it activates the nonce from the new page's csp-nonce meta tag. A
  # per-request nonce therefore reports every inline script reached by a
  # Turbo visit (the system test "a Turbo visit to a page with an inline
  # script raises no violations" fails with SecureRandom.base64(16) here).
  # The session id is hashed so it never appears in the page, and a session
  # without an id yet gets a random nonce.
  config.content_security_policy_nonce_generator = ->(request) do
    session_id = request.session.id.to_s
    session_id.present? ? Digest::SHA256.base64digest("csp-nonce:#{session_id}") : SecureRandom.base64(16)
  end
  config.content_security_policy_nonce_directives = %w[ script-src ]

  config.content_security_policy_report_only = true
end
