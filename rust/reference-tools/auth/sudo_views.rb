# Sudo page bodies rendered from the d7c7de92 WS19 default seed. Use the same deterministic
# form-token renderer as reference-tools/views/core/goldens.rb; no application behavior mocked.
require "json"
class GoldenSudosController < SudosController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
env = { http_host: "campfire.test", https: false, "rack.session" => {} }
renderer = GoldenSudosController.renderer.new(env)
Current.reset
goldens = {}
[ [], [:password], [:totp], [:google], [:password, :totp, :google] ].each do |verifiers|
  goldens[verifiers.join("_")] = renderer.render(template: "sudos/new", layout: false, assigns: { verifiers: })
end
params = { "user" => { "name" => "David <&>", "tags" => ["a", true, false, nil, 12, {"skip" => 1}], "empty" => nil }, "invalid name" => "skip", "bad]name" => "allowed" }
goldens["continue"] = renderer.render(template: "sudos/continue", layout: false, assigns: {
  sudo_replay_path: "/account/users/127326141?x=1&y=2", sudo_replay_method: "patch", sudo_replay_params: params
})
File.write(ENV.fetch("WS9_SUDO_GOLDENS"), JSON.pretty_generate(goldens) + "\n")
puts "WS9 sudo views: #{goldens.size} Rails page bodies rendered from the parity seed"
