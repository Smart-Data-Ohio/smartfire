# Actual pinned Rails helpers/templates with WS6's deterministic CSRF convention.
require "json"
class GoldenSessionsController < Users::SessionsController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
renderer = GoldenSessionsController.renderer.new(http_host: "campfire.test", https: false, "rack.session" => {})
user = User.find(712064548)
chrome = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
firefox = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0"
current = Session.new(id: 333, user: user, user_agent: chrome, ip_address: "192.0.2.10", created_at: 1.day.ago, last_active_at: Time.current)
other = Session.new(id: 334, user: user, user_agent: firefox, ip_address: "192.0.2.20", created_at: 3.days.ago, last_active_at: 2.days.ago)
Current.user = user
Current.session = current
goldens = {
  two: renderer.render(template: "users/sessions/index", layout: false, assigns: { sessions: [current, other] }),
  one: renderer.render(template: "users/sessions/index", layout: false, assigns: { sessions: [current] }),
  profile: renderer.render(partial: "users/profiles/sessions"),
  sessions: [current, other].map { |s| { id: s.id, current: s == current, description: s.device_description, ip_address: s.ip_address, created_at: s.created_at.iso8601, last_active_at: s.last_active_at.iso8601 } }
}
File.write(ENV.fetch("WS9_SESSION_GOLDENS"), JSON.pretty_generate(goldens) + "\n")
puts "WS9 session views: 2 Rails page bodies and the profile sessions panel rendered from the parity seed"
