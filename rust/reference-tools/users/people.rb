# Real pinned Rails models and views, with the same fixed token/nonce inputs as WS6.
require "json"
require "digest"
require "set"
Rails.logger = ActiveSupport::Logger.new($stderr)
%w[app/controllers/users_controller.rb app/controllers/users/cards_controller.rb app/views/users/index.html.erb app/views/users/cards/show.html.erb app/views/users/statuses/_badge.html.erb app/views/users/stars/_toggle.html.erb].each do |file|
  expected = JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"), "reference-tools/users/people-source-hashes.json"))).fetch(file)
  raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest == expected
end
load File.join(ENV.fetch("PARITY_WORK"), "reference-tools/users/post_pin.rb")
class PeopleGoldenController < ApplicationController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
def render_people(template, viewer, assigns)
  Current.user = viewer
  PeopleGoldenController.renderer.new(http_host: "campfire.test", https: false,
    "rack.session" => {}, "action_dispatch.content_security_policy_nonce_generator" => ->(_) { "NONCE" })
    .render(template: template, layout: false, assigns: assigns)
ensure
  Current.reset
end
david = User.find(127326141)
jason = User.find(149087659)
kevin = User.find(712064548)
bender = User.find(394959859)
cases = [
  ["review_markup", kevin, {attributes:{name:'<b>Person & "</b>',bio:'<script>bio()</script>',custom_status_text:'<img src=x onerror="status()">',custom_status_emoji:'<b>&</b>',custom_status_expires_at:1.hour.from_now}}],
  ["peer_online", jason, { lease: true }], ["peer_offline", kevin, {}],
  ["custom_status", jason, { lease: true, attributes: { custom_status_emoji: "🚂", custom_status_text: "On a train", custom_status_expires_at: 1.hour.from_now } }],
  ["own", david, {}], ["agent", bender, {}],
  ["deactivated", kevin, { attributes: { status: 1 } }], ["banned", kevin, { attributes: { status: 2 } }],
  ["starred", kevin, { star: true }], ["idle", jason, { lease: true, idle: true }],
  ["invisible", jason, { lease: true, attributes: { presence_setting: "invisible" } }],
  ["dnd", jason, { lease: true, attributes: { presence_setting: "dnd" } }],
  ["expired_status", kevin, { attributes: { custom_status_text: "gone", custom_status_expires_at: Time.current } }],
  ["unmentionable", kevin, { attributes: { name: "Name[with bracket]" } }]
].map do |name, target, setup|
  result = nil
  ActiveRecord::Base.transaction(requires_new: true) do
    target.update_columns(**setup.fetch(:attributes, {})) if setup[:attributes]
    if setup[:lease]
      session = target.sessions.first || target.sessions.create!(user_agent: "ws8br2", ip_address: "127.0.0.1")
      lease = WorkspacePresenceLease.establish(user: target, session: session)
      lease.update_columns(last_active_at: 11.minutes.ago) if setup[:idle]
    end
    david.user_stars.create!(starred_user: target) if setup[:star]
    html = render_people("users/cards/show", david, user: target.reload, starred: david.starred?(target))
    result = { name: name, user_id: target.id, setup: setup.deep_transform_values { |value| value.respond_to?(:iso8601) ? value.iso8601(6) : value }, html: html }
    raise ActiveRecord::Rollback
  end
  result
end
directories = [false, true].map do |star|
  result = nil
  ActiveRecord::Base.transaction(requires_new: true) do
    david.user_stars.create!(starred_user: kevin) if star
    people = User.active.includes(:agent).with_attached_avatar.ordered.where.not(id: david.id).to_a
    starred = david.starred_ids_among(people.map(&:id))
    people = people.partition { |u| starred.include?(u.id) }.flatten
    html = render_people("users/index", david, users: people, starred_ids: starred, online_ids: WorkspacePresenceLease.online_user_ids(people.map(&:id)).to_set)
    result = { starred: star, html: html, ids: people.map(&:id) }
    raise ActiveRecord::Rollback
  end
  result
end
puts JSON.pretty_generate(reference: ENV.fetch('PARITY_REFERENCE_SHA'), cards: cases, directories: directories)
warn "Rails people oracle: #{cases.size} cards, #{directories.size} directories; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
