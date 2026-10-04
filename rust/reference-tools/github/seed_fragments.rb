# Fragment-only contracts for WS8b-m, WS8b-r2 and WS11-ui, on real default-seed records.
require "json"
require "digest"
class GithubFragmentController < ApplicationController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
ActiveJob::Base.queue_adapter = :test
paths = %w[app/views/github/pull_requests/_thread_header.html.erb app/views/github/pull_requests/_write_actions.html.erb app/views/users/profiles/_github_connection.html.erb app/views/accounts/bots/edit.html.erb]
sources = paths.to_h { |p| [p, Digest::SHA256.file(Rails.root.join(p)).hexdigest] }
renderer = GithubFragmentController.renderer.new(http_host: "campfire.test", https: false, "rack.session" => {})
user = User.find(127326141)
bot = User.find(394959859)
Current.user = user
mapping = Github::PullRequestThread.find_by!(channel_thread_id: 8)
thread, pr = mapping.channel_thread, mapping.pull_request
# The bot section is inline in Rails. Render its unchanged source, not a reconstructed template.
bot_source = File.read(Rails.root.join("app/views/accounts/bots/edit.html.erb"))
bot_source = bot_source[bot_source.index('  <section aria-labelledby="github-connection-title">')..]
bot_source = bot_source[0...bot_source.index("  </section>")+"  </section>\n".length]
thread_fragments = {
  thread_id: thread.id, room_id: thread.room_id, pull_request_id: pr.id,
  header: renderer.render(partial: "github/pull_requests/thread_header", locals: { thread:, pull_request: pr }),
  write_frame: renderer.render(partial: "github/pull_requests/write_actions", locals: { thread:, room: thread.room, pull_request: pr })
}
cases = []
[false, true].each do |app|
  ENV["GITHUB_APP_CLIENT_ID"] = app ? "fixture-client" : nil
  ENV["GITHUB_APP_CLIENT_SECRET"] = app ? "fixture-secret" : nil
  [:unlinked, :pat, :app, :disconnected, :blank_reason, :escape].each do |state|
    [user, bot].each do |actor|
      actor.github_connected_account&.destroy!
      unless state == :unlinked
        actor.create_github_connected_account!(github_login: state == :escape ? "<login>&" : "parity-user", access_token: "fragment-fixture-token", token_source: state == :app ? "app" : "pat", disconnected_reason: [:disconnected, :blank_reason].include?(state) ? (state == :blank_reason ? " " : "Revoked <grant>&") : nil)
      end
      actor.reload
    end
    account = user.github_connected_account
    [true, false].each do |admin|
      Current.user = admin ? user : User.find(712064548)
      cases << {
        name: "#{state}_#{app}_#{admin}", app_configured: app, administrator: admin,
        data: { linked: !!account, usable: account&.usable? || false, login: account&.github_login || "", reason: account&.disconnected_reason, app_token: account&.app_token? || false },
        profile: renderer.render(partial: "users/profiles/github_connection", locals: { user: }),
        bot: renderer.render(inline: bot_source, layout: false, assigns: { bot: })
      }
    end
  end
end
Current.reset
File.write(ENV.fetch("GITHUB_SEED_FRAGMENTS"), JSON.pretty_generate({ reference: ENV.fetch("PARITY_REFERENCE_SHA"), sources:, bot_id: bot.id, user_id: user.id, thread: thread_fragments, connections: cases })+"\n")
puts "GitHub seed fragments Rails oracle: thread header/write frame and #{cases.size} profile/bot states; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
