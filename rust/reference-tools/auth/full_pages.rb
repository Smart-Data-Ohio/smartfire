# Complete owned pages, using the pinned application layout and real parity-seed records.
# WS6's golden convention fixes request-generated tokens at the renderer boundary.
require "json"
class GoldenAuthController < SudosController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
env = {
  http_host: "campfire.test", https: false,
  "HTTP_USER_AGENT" => "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
  "rack.session" => {},
  "action_dispatch.content_security_policy" => Rails.application.config.content_security_policy,
  "action_dispatch.content_security_policy_nonce_generator" => ->(_request) { "NONCE" }
}
renderer = GoldenAuthController.renderer.new(env)
Current.reset
user = User.find(712064548)
credential = TwoFactorCredential.new(user: user, secret: "JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP")
uri = credential.provisioning_uri
user.reload # discard the inverse transient credential association
chrome = env.fetch("HTTP_USER_AGENT")
firefox = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0"
current = Session.new(id: 333, user: user, user_agent: chrome, ip_address: "192.0.2.10", created_at: 1.day.ago, last_active_at: Time.current)
other = Session.new(id: 334, user: user, user_agent: firefox, ip_address: "192.0.2.20", created_at: 3.days.ago, last_active_at: 2.days.ago)
codes = %w[abcde12345 fghij67890 klmno12345 pqrst67890 uvwxy12345 zabcd67890 efghi12345 jklmn67890 opqrs12345 tuvwx67890]
params = { "user" => { "name" => "David <&>", "tags" => ["a", true, false, nil, 12, {"skip" => 1}], "empty" => nil }, "invalid name" => "skip", "bad]name" => "allowed" }
cases = [
  ["sign_in", nil, "sessions/new", {}],
  ["incompatible_browser", nil, "sessions/incompatible_browser", {}],
  ["transfer", nil, "sessions/transfers/show", {}],
  ["setup", user, "two_factor/setups/show", { credential:, provisioning_uri: uri }],
  ["challenge", nil, "two_factor/challenges/show", {}],
  ["backups", user, "two_factor/backup_codes/show", { backup_codes: codes, continue_url: "http://campfire.test/users/me/profile", signed_out_other_devices: 0 }],
  ["backups_signed_out", user, "two_factor/backup_codes/show", { backup_codes: codes, continue_url: "http://campfire.test/rooms/486777696", signed_out_other_devices: 2 }],
  ["sessions_one", user, "users/sessions/index", { sessions: [current] }],
  ["sessions_two", user, "users/sessions/index", { sessions: [current, other] }],
  ["sudo_continue", user, "sudos/continue", { sudo_replay_path: "/account/users/127326141?x=1&y=2", sudo_replay_method: "patch", sudo_replay_params: params }]
]
[ [], [:password], [:totp], [:google], [:password, :totp, :google] ].each do |verifiers|
  cases << ["sudo_#{verifiers.join('_')}", user, "sudos/new", { verifiers: }]
end
pages = cases.to_h do |name, signed_in, template, assigns|
  Current.reset
  Current.user = signed_in
  Current.session = signed_in ? current : nil
  page_renderer = name == "transfer" ? GoldenAuthController.renderer.new(env.merge("PATH_INFO" => "/session/transfers/some-token", "action_dispatch.request.path_parameters" => { controller: "sessions/transfers", action: "show", id: "some-token" })) : renderer
  [name, page_renderer.render(template:, layout: "application", assigns:)]
end
Current.user = user
File.write(ENV.fetch("WS9_FULL_PAGE_GOLDENS"), JSON.pretty_generate({
  pages:, user_id: user.id, uri:, key: credential.formatted_secret, codes:, replay: params,
  brand_icon_names: Icons.client_icon_names,
  user: { avatar_path: renderer.render(inline: "<%= fresh_user_avatar_path(user) %>", locals: { user: }, layout: false),
    theme: user.theme, text_size: user.text_size, time_zone: user.time_zone,
    time_zone_explicit: user.time_zone_explicit, tour_completed: user.tour_completed_at.present?,
    voice_mode: user.voice_mode, push_to_talk_key: user.push_to_talk_key },
  sessions: [current, other].map { |s| { id: s.id, current: s == current, description: s.device_description, ip_address: s.ip_address, created_at: s.created_at.iso8601, last_active_at: s.last_active_at.iso8601 } }
}) + "\n")
puts "WS9 full-page goldens: #{pages.size} complete Rails auth pages rendered from the parity seed"
