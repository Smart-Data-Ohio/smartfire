# User-star rows and rendered fragments from our pinned Rails application.
require "json"
require "digest"
Rails.logger = ActiveSupport::Logger.new($stderr)
Rails.application.config.hosts.clear
FILES = %w[app/models/user_star.rb app/models/user/starring.rb app/controllers/users/stars_controller.rb app/views/users/stars/_toggle.html.erb].freeze
hashes = JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "reference-tools/users/stars-source-hashes.json")))
FILES.each { |file| raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest == hashes.fetch(file) }
class StarsGoldenController < ApplicationController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
david, jason, kevin, bender = [127326141, 149087659, 712064548, 394959859].map { |id| User.find(id) }
fragments = [false, true].map do |starred|
  Current.user = david
  html = StarsGoldenController.renderer.new(http_host: "campfire.test", https: false, "rack.session" => {})
    .render(partial: "users/stars/toggle", locals: {user: kevin, starred: starred})
  stream = Turbo::Streams::TagBuilder.new(StarsGoldenController.new.view_context)
    .replace("star_user_#{kevin.id}", html)
  Current.reset
  {starred: starred, html: html, stream: stream.to_s}
end
validations = [[david, david], [bender, bender], [david, nil], [nil, kevin]].map do |viewer, target|
  star = UserStar.new(user: viewer, starred_user: target)
  star.valid?
  {user_id: viewer&.id, starred_user_id: target&.id, errors: star.errors.to_hash}
end
http = []
[david, bender, nil].each do |viewer|
  browser = ActionDispatch::Integration::Session.new(Rails.application)
  browser.host! "campfire.test"
  if viewer
    request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
    signed_in = viewer.sessions.first || viewer.sessions.create!(user_agent: "ws12", ip_address: "127.0.0.1", two_factor_verified_at: Time.current)
    request.cookie_jar.signed.permanent[:session_token] = {value: signed_in.token, httponly: true, same_site: :lax}
    browser.cookies["session_token"] = request.cookie_jar[:session_token]
  end
  # Controller response bytes don't depend on CSRF; real CSRF refusal is tested separately.
  previous = ActionController::Base.allow_forgery_protection
  ActionController::Base.allow_forgery_protection = false
  [["post",kevin],["post",kevin],["delete",kevin],["delete",kevin],["post",bender],["post",viewer&.id || david.id],["post",-1]].each do |method, target|
    target = target.id if target.respond_to?(:id)
    browser.public_send(method, "/users/#{target}/star", as: :json)
    ActiveSupport::IsolatedExecutionState.clear
    http << {viewer_id: viewer&.id, method: method, target: target, status: browser.response.status,
      content_type: browser.response.headers["Content-Type"], body: browser.response.body,
      count: UserStar.where(user_id: viewer&.id, starred_user_id: target).count}
  end
ensure
  ActionController::Base.allow_forgery_protection = previous
end
puts JSON.pretty_generate(reference: "d7c7de92", fragments: fragments, validations: validations, http: http)
warn "Rails stars oracle: #{fragments.size} fragments and streams, #{validations.size} validations, #{http.size} HTTP responses; reference d7c7de92"
