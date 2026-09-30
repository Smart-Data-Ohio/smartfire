# Goldens for the views foundation (crates/views/tests/golden/core), rendered by our Rails:
# the application and public layouts with their partials, shared/*, the mailer layouts, the
# worked-example pages, and helper vectors (time zones and formats, time_ago_in_words, avatars,
# translations, fragment cache keys).
#
# Run by reference-tools/views/core/run.sh on a fresh database with the test fixtures loaded and
# the clock frozen, so every timestamp, signed id and avatar URL is reproducible. Writes
# goldens.json to the storage's db directory (the only writable mount).
#
# Session-bound values are fixed placeholders: every form_authenticity_token is "GLOBAL" (the
# meta tag) or "<method>:<action>" (per-form), and the CSP nonce is "NONCE". The Rust tests lend
# the templates the same values (campfire_views::helpers::request_forgery::rendering_with).
require "json"
require "ostruct"
require_relative "message_icons"

ActiveRecord::Base.logger = nil

# `fixtures :all` with test_helper.rb's fixture classes.
require "active_record/fixtures"
fixtures = Rails.root.join("test/fixtures")
ActiveRecord::FixtureSet.create_fixtures(fixtures,
  Dir[fixtures.join("**/*.yml")].map { |path| path.delete_prefix("#{fixtures}/").delete_suffix(".yml") },
  { "twitter_posts" => Twitter::Post, "twitter_post_references" => Twitter::PostReference })

OUT = "/rails/storage/db/goldens.json"
HOST = "campfire.test"
USER_AGENT = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36"

class GoldenController < ApplicationController
  # "GLOBAL" for csrf_meta_tags, "<method>:<action>" for a form's per-form token.
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end

class GoldenSearchesController < SearchesController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end

  def self.controller_name = "searches"
end

def user(email) = User.find_by!(email_address: email)

def renderer_env(flash: {}, query: nil)
  {
    http_host: HOST,
    https: false,
    "HTTP_USER_AGENT" => USER_AGENT,
    "QUERY_STRING" => query.to_s,
    "rack.session" => {},
    "action_dispatch.content_security_policy" => Rails.application.config.content_security_policy,
    "action_dispatch.content_security_policy_nonce_generator" => ->(_request) { "NONCE" },
    "action_dispatch.request.flash_hash" => ActionDispatch::Flash::FlashHash.new(flash.stringify_keys)
  }
end

def render_with(controller: GoldenController, user: nil, flash: {}, query: nil, **options)
  Current.reset
  Current.user = user
  controller.renderer.new(renderer_env(flash:, query:)).render(**options)
ensure
  Current.reset
end

# A page body that fills every content_for region the application layout yields.
FULL_BODY = <<~ERB
  <% content_for :head do %>
    <meta name="golden-head" content="head">
  <% end %>
  <% content_for :nav do %>
    <span class="golden-nav">Nav</span>
  <% end %>
  <% content_for :sidebar do %>
    <div class="golden-sidebar">Sidebar</div>
  <% end %>
  <% content_for :footer do %>
    <div class="golden-footer">Footer</div>
  <% end %>
  <% content_for :member_panel do %>
    <aside class="golden-member-panel"></aside>
  <% end %>
  <% content_for :thread_panel do %>
    <aside class="golden-thread-panel"></aside>
  <% end %>
  <p class="golden-body">Body</p>
ERB

MINIMAL_BODY = "<p class=\"golden-body\">Body</p>\n"

def with_env(overrides)
  saved = overrides.keys.to_h { |key| [ key, ENV[key] ] }
  overrides.each { |key, value| ENV[key] = value }
  yield
ensure
  saved.each { |key, value| ENV[key] = value }
end

HUDDLE_ENV = {
  "LIVEKIT_URL" => "wss://livekit.campfire.test",
  "LIVEKIT_INTERNAL_URL" => "http://livekit:7880",
  "LIVEKIT_API_KEY" => "key",
  "LIVEKIT_API_SECRET" => "secret",
  "LIVEKIT_GATEWAY_SECRET" => "gateway"
}.freeze

GOOGLE_ENV = {
  "GOOGLE_CLIENT_ID" => "client-id.apps.googleusercontent.com",
  "GOOGLE_PICKER_API_KEY" => "picker-api-key",
  "GOOGLE_CLOUD_PROJECT_NUMBER" => "123456789012"
}.freeze

goldens = { "layouts" => {}, "partials" => {}, "pages" => {}, "helpers" => {}, "facts" => {} }

david = user("david@37signals.com")
jz = user("jz@37signals.com")
kevin = user("kevin@37signals.com")
bender = User.find_by!(name: "Bender Bot")

# ---------------------------------------------------------------------------------------------
# Layout states

layouts = goldens["layouts"]

layouts["application_signed_out"] =
  render_with(user: nil, inline: MINIMAL_BODY, layout: "application")

layouts["application_member"] =
  render_with(user: jz, inline: MINIMAL_BODY, layout: "application")

layouts["application_bot"] =
  render_with(user: bender, inline: MINIMAL_BODY, layout: "application")

# Every region filled, a title and body class, a notice, an administrator with a theme, a
# text size and a saved zone.
david.assign_attributes(theme: "dark", text_size: "large", time_zone: "Eastern Time (US & Canada)")
layouts["application_admin_full"] =
  render_with(user: david, inline: FULL_BODY, layout: "application",
    assigns: { page_title: "Golden <Title> & Co", body_class: "golden-page" }, flash: { notice: "Saved <b>it</b>" })
david.restore_attributes

# An alert, huddles and Google configured, every notification sound meta tag, the tour pending,
# "Not set" chosen explicitly as the zone, and push-to-talk.
kevin.assign_attributes(
  theme: "light", text_size: "smaller", time_zone: nil, time_zone_explicit: true, tour_completed_at: nil,
  dnd_enabled: true, quiet_hours_enabled: true, quiet_hours_start_minute: 1320, quiet_hours_end_minute: 420,
  meeting_dnd_enabled: true, meeting_status_enabled: true, ooo_notify_enabled: false, ooo_calendar_enabled: true,
  ooo_until: Time.current + 2.days, voice_mode: "push_to_talk", push_to_talk_key: "KeyT"
)
kevin.define_singleton_method(:meeting_cache) do
  OpenStruct.new(quiet_window_epochs: [ [ 1_790_000_000, 1_790_003_600 ], [ 1_790_010_000, 1_790_012_000 ] ],
    ooo_window_epochs: [ [ 1_790_100_000, 1_790_200_000 ] ])
end
kevin.define_singleton_method(:google_account) { OpenStruct.new(drive?: true) }
with_env(HUDDLE_ENV.merge(GOOGLE_ENV)) do
  layouts["application_everything_on"] =
    render_with(user: kevin, inline: FULL_BODY, layout: "application", flash: { alert: "Couldn't save" })
end

# The global search on the searches page keeps the query; david has recent searches.
layouts["application_searches_query"] =
  render_with(controller: GoldenSearchesController, user: david, query: "q=%20pizza%20%20night%20",
    inline: MINIMAL_BODY, layout: "application")

goldens["facts"]["brand_icon_names"] = Icons.client_icon_names.join(",")
goldens["facts"]["users"] = [ david, jz, kevin, bender ].to_h do |u|
  u.reload
  [ u.name, {
    "id" => u.id, "name" => u.name, "administrator" => u.administrator?, "bot" => u.bot?,
    "avatar_path" => render_with(inline: "<%= fresh_user_avatar_path(u) %>", locals: { u: u }),
    "time_zone" => u.time_zone, "tour_completed" => !u.tour_completed_at.nil?,
    "searches" => u.searches.ordered.limit(10).map { |s| { "id" => s.id, "query" => s.query } }
  } ]
end
goldens["facts"]["account"] = {
  "name" => Account.first.name,
  "logo_path" => render_with(inline: "<%= fresh_account_logo_path %>"),
  "custom_styles" => Account.first.custom_styles.to_s
}

# ---------------------------------------------------------------------------------------------
# Public and mailer layouts

layouts["public"] = render_with(inline: MINIMAL_BODY, layout: "public", assigns: { page_title: "Golden" })
layouts["public_described"] =
  render_with(inline: MINIMAL_BODY, layout: "public", assigns: { page_description: "About <this> & that" })
layouts["mailer_html"] = render_with(inline: "<p>Hello</p>\n", layout: "mailer")
layouts["mailer_text"] = render_with(inline: "Hello\n", layout: "mailer", formats: [ :text ])

# ---------------------------------------------------------------------------------------------
# Shared partials

partials = goldens["partials"]
partials["shared_multi_select_bar"] = render_with(user: david, partial: "shared/multi_select_bar")
partials["shared_multi_select_bar_exit"] =
  render_with(user: david, partial: "shared/multi_select_bar", locals: { exit_button: true })

icon_field = <<~ERB
  <%= form_with model: record, url: "/golden", scope: scope do |form| %>
    <%= render "shared/icon_field", form: form, record: record, scope: scope %>
  <% end %>
ERB
room = Room.first
[ [ "none", nil ], [ "brand", "github" ], [ "emoji", "tada" ], [ "unknown", "no_such_icon" ] ].each do |label, icon|
  room.icon_name = icon
  partials["shared_icon_field_#{label}"] =
    render_with(user: david, inline: icon_field, locals: { record: room, scope: "room" })
end
room.restore_attributes

partials["searches_dropdown_recents_empty"] =
  render_with(user: david, partial: "searches/dropdown_recents", locals: { searches: [] })
partials["first_paint_controller_preloads"] =
  render_with(user: david, inline: "<%= first_paint_controller_preloads %>")
goldens["helpers"]["avatars"] = [ david, bender ].map do |u|
  { "user" => u.name, "title" => u.title,
    "html" => render_with(inline: '<%= avatar_tag u, size: 32, loading: "lazy" %>', locals: { u: u }) }
end

# ---------------------------------------------------------------------------------------------
# Worked-example pages, as real responses

require "/work/reference-tools/views/core/review.rb"
generate_review_goldens(goldens)

session = ActionDispatch::Integration::Session.new(Rails.application)
session.host! HOST
session.https!
{
  "public_pages_about" => "/about",
  "public_pages_privacy" => "/privacy",
  "public_pages_terms" => "/terms",
  "pwa_manifest" => "/webmanifest.json",
  "pwa_service_worker" => "/service-worker.js"
}.each do |name, path|
  session.get path, headers: { "User-Agent" => USER_AGENT }
  goldens["pages"][name] = {
    "path" => path, "status" => session.response.status,
    "content_type" => session.response.headers["content-type"], "body" => session.response.body
  }
end

# ---------------------------------------------------------------------------------------------
# Helper vectors

helpers = goldens["helpers"]
view = GoldenController.new.view_context

# Time zones: every zone Rails names, its identifier, and how lookups resolve.
helpers["zones"] = ActiveSupport::TimeZone.all.map { |zone| [ zone.name, zone.tzinfo.identifier ] }
helpers["zone_mapping"] = ActiveSupport::TimeZone::MAPPING.to_a
# What TZInfo (the image's system zoneinfo) accepts as an identifier.
helpers["tz_identifiers"] = TZInfo::Timezone.all_identifiers.sort
helpers["tz_data_source"] = TZInfo::DataSource.get.to_s
helpers["zone_lookups"] = [
  "UTC", "Etc/UTC", "Eastern Time (US & Canada)", "America/New_York", "america/new_york", "London",
  "Europe/London", "Asia/Kolkata", "Kolkata", "Mumbai", "Pacific/Chatham", "Chatham Is.", "Nuku'alofa",
  "America/Argentina/Buenos_Aires", "Buenos Aires", "GMT", "EST", "Nowhere/Special", "", " UTC"
].map { |name| [ name, ActiveSupport::TimeZone[name]&.name, ActiveSupport::TimeZone[name]&.tzinfo&.identifier ] }

instants = [
  Time.utc(2026, 1, 5, 9, 3, 7), Time.utc(2026, 3, 8, 6, 59, 59), Time.utc(2026, 3, 8, 7, 0, 0),
  Time.utc(2026, 7, 4, 23, 30, 0), Time.utc(2026, 11, 1, 5, 30, 0), Time.utc(2026, 12, 31, 23, 59, 59),
  Time.utc(2027, 2, 28, 12, 0, 0)
]
zones = [ "UTC", "Eastern Time (US & Canada)", "America/New_York", "Asia/Kolkata", "Pacific/Chatham",
  "Australia/Adelaide", "London", "Hawaii", "Arizona", "International Date Line West", "Nuku'alofa", "Kathmandu" ]
strftimes = [ "%B %-d, %Y at %-I:%M %p", "%B %-d, %Y at %-I:%M %p %Z", "%B %-d, %Y", "%Y-%m-%dT%H:%M",
  "%b %-d, %Y, %-I:%M %p", "%b %-d", "%-I:%M %p", "%A, %B %-d", "%a %e %b %H:%M:%S %z %Y", "%l:%M%P", "%d/%m/%y %j" ]
helpers["times"] = zones.flat_map do |zone_name|
  zone = ActiveSupport::TimeZone[zone_name]
  instants.map do |instant|
    time = instant.in_time_zone(zone)
    {
      "zone" => zone_name, "utc" => instant.iso8601,
      "iso8601" => time.iso8601, "to_s" => time.to_s, "short" => time.to_fs(:short), "long" => time.to_fs(:long),
      "db" => time.to_fs(:db), "number" => time.to_fs(:number), "epoch" => time.to_fs(:epoch),
      "date_long" => time.to_date.to_fs(:long), "date_short" => time.to_date.to_fs(:short), "date" => time.to_date.to_s,
      "zone_abbreviation" => time.zone, "formatted_offset" => time.formatted_offset,
      "strftime" => strftimes.to_h { |format| [ format, time.strftime(format) ] },
      "local_datetime_tag" => view.local_datetime_tag(time),
      "local_datetime_tag_date" => view.local_datetime_tag(time, style: :date, class: "golden") { "then" }
    }
  end
end

# distance_of_time_in_words, from a fixed instant, both ways, with and without seconds.
base = Time.utc(2026, 2, 10, 12, 0, 0)
spans = [ 0, 1, 4, 5, 9, 10, 19, 20, 29, 30, 39, 40, 44, 45, 59, 60, 89, 90, 91, 119, 120, 150, 29 * 60 + 29, 29 * 60 + 30,
  44 * 60 + 29, 44 * 60 + 30, 89 * 60 + 29, 89 * 60 + 30, 90 * 60, 23.hours + 59.minutes + 29, 23.hours + 59.minutes + 30,
  1.day, 41.hours + 59.minutes + 29, 41.hours + 59.minutes + 30, 2.days, 29.days + 23.hours + 59.minutes + 29,
  29.days + 23.hours + 59.minutes + 30, 44.days + 23.hours + 59.minutes + 29, 44.days + 23.hours + 59.minutes + 30,
  59.days + 23.hours + 59.minutes + 29, 59.days + 23.hours + 59.minutes + 30, 100.days, 364.days, 365.days, 366.days,
  1.year + 3.months, 1.year + 3.months + 1.day, 1.year + 9.months, 1.year + 9.months + 1.day, 2.years, 3.years + 5.months,
  4.years, 8.years + 11.months, 30.years, 400.years ].map(&:to_i)
helpers["distance_of_time"] = spans.flat_map do |seconds|
  [ [ base, base + seconds ], [ base + seconds, base ] ].map do |from, to|
    {
      "from" => from.iso8601, "to" => to.iso8601,
      "words" => view.distance_of_time_in_words(from, to),
      "words_with_seconds" => view.distance_of_time_in_words(from, to, include_seconds: true)
    }
  end
end

names = [ "David Heinemeier Hansson", "jz", "Élodie Martin", "  ", "Bender Bot", "O'Brien-Smith", "李 小龍", "a_b c", "37signals HQ" ]
helpers["initials"] = names.map { |name| [ name, User.new(name: name).initials ] }
helpers["avatar_colors"] = [ 1, 2, 3, 42, 980190962, 298486374 ].map { |id| [ id, view.avatar_background_color(User.new(id: id)) ] }

helpers["translations"] = TranslationsHelper::TRANSLATIONS.keys.to_h do |key|
  [ key.to_s, { "entries" => TranslationsHelper::TRANSLATIONS[key].map { |language, text| [ language.to_s, text ] },
    "translations_for" => view.translations_for(key), "translation_button" => view.translation_button(key) } ]
end
helpers["translation_table"] = TranslationsHelper::TRANSLATIONS
helpers["reactions"] = EmojiHelper::REACTIONS.to_a

helpers["signed_stream_names"] = [
  [ [ "rooms" ], view.turbo_stream_from(:rooms) ],
  [ [ "gid", david.to_gid_param, "rooms" ], view.turbo_stream_from(david, :rooms) ]
]

# ---------------------------------------------------------------------------------------------
# Fragment cache keys (ActiveSupport::Cache.expand_cache_key of each key).

cache = goldens["cache_keys"] = {}
cache["records"] = Message.order(:id).limit(3).map { |message| [ message.id, message.cache_key_with_version ] }
cache["expand"] = [
  [ "nil", nil ], [ "true", true ], [ "false", false ], [ "integer", 2 ], [ "string", "a b/c" ],
  [ "time", Time.utc(2026, 2, 10, 12, 0, 0, 123456) ], [ "zoned_time", Time.utc(2026, 2, 10, 12, 0, 0).in_time_zone("Hawaii") ],
  [ "array", [ 1, nil, [ 2, [ 3, nil ] ], true, "x" ] ], [ "empty_array", [] ],
  [ "record_and_friends", [ Message.order(:id).first, nil, [ [ 7, Time.utc(2026, 1, 1) ] ], 3, false, 2 ] ]
].map { |label, key| [ label, ActiveSupport::Cache.expand_cache_key(key) ] }


# Refuse an old image: this must exercise the real helper, without repairing its behavior here.
raise "Reference must include Rails PR #148" unless MessagesHelper::PRESENTATION_CACHE_VERSION == 3
cache["presentation_cache_version"] = MessagesHelper::PRESENTATION_CACHE_VERSION

# The key is built from associations; each case stubs them on a real message (and the view's
# queries) so that every branch of the helper is exercised with fixed stamps. Times are
# TimeWithZone in Time.zone, as Active Record returns them, and the key is expanded under two
# zones: a Time element expands through Time#to_a, so the zone shows in the key.
def utc_iso(time) = time&.utc&.iso8601(6)

message_cases = [
  { "label" => "plain" },
  { "label" => "everything",
    "cards" => [ "2026-01-02T03:04:05.123456Z", "2026-01-09T10:00:00.000001Z", nil ],
    "embeds" => [ [ 5, "2026-01-03T00:00:00.5Z" ], [ 6, nil ] ],
    "has_pull_requests" => true, "pr_threads_stamp" => "2026-01-04T05:06:07Z",
    "pins" => [ "2026-01-05T00:00:00Z", "2026-01-06T00:00:00Z" ], "thread_messages_count" => 7,
    "poll" => "2026-01-07T08:09:10Z", "system_note" => true, "streaming" => true,
    "agent_steps" => [ "2026-01-08T00:00:00Z" ],
    "quoted_sources" => [ { "updated_at" => "2026-01-01T00:00:00Z", "edited_at" => "2026-01-10T00:00:00Z", "creator" => "Élodie \"E\" Martin", "room" => "Designers" },
                          { "updated_at" => "2026-01-02T00:00:00Z", "edited_at" => nil, "creator" => "jz", "room" => "Hash \#{x} \\ tab\t" } ] },
  { "label" => "pull_request_without_thread", "has_pull_requests" => true, "pr_threads_stamp" => nil, "cards" => [ "2026-03-08T07:00:00Z" ] },
  { "label" => "empty_thread", "thread_messages_count" => 0, "pins" => [], "agent_steps" => [] },
  { "label" => "quote_of_unedited_source", "quoted_sources" => [ { "updated_at" => "2026-02-01T00:00:00Z", "edited_at" => nil, "creator" => "Bender Bot", "room" => "All Pets" } ] },
  { "label" => "quote_updated_after_edit", "quoted_sources" => [ { "updated_at" => "2026-02-02T00:00:00Z", "edited_at" => "2026-02-01T00:00:00Z", "creator" => "David", "room" => "HQ" } ] }
]
fragment_message = Message.order(:id).first
parse = ->(iso) { iso && Time.iso8601(iso).in_time_zone }
cache["message_with_pr_cards"] = message_cases.map do |kase|
  expanded = [ "UTC", "Hawaii" ].to_h do |zone_name|
    Time.use_zone(zone_name) do
      message = Message.find(fragment_message.id)
      stamps = ->(key) { (kase[key] || []).map { |iso| OpenStruct.new(updated_at: parse.(iso)) } }
      message.define_singleton_method(:github_pull_requests) { kase["has_pull_requests"] ? stamps.("cards").presence || [ OpenStruct.new(updated_at: nil) ] : [] }
      message.define_singleton_method(:fizzy_cards) { kase["has_pull_requests"] ? [] : stamps.("cards") }
      message.define_singleton_method(:twitter_posts) { [] }
      message.define_singleton_method(:events) { [] }
      message.define_singleton_method(:link_embed_references) { (kase["embeds"] || []).map { |id, iso| OpenStruct.new(id: id, link_embed: OpenStruct.new(updated_at: parse.(iso))) } }
      message.define_singleton_method(:message_pins) { stamps.("pins") }
      message.define_singleton_method(:channel_thread) { kase["thread_messages_count"] && OpenStruct.new(messages_count: kase["thread_messages_count"]) }
      message.define_singleton_method(:poll) { kase["poll"] && OpenStruct.new(updated_at: parse.(kase["poll"])) }
      message.define_singleton_method(:system_note?) { kase["system_note"] || false }
      message.define_singleton_method(:streaming?) { kase["streaming"] || false }
      message.define_singleton_method(:agent_steps) { stamps.("agent_steps") }
      helper = GoldenController.new.view_context
      helper.define_singleton_method(:github_pr_threads_stamp) { |_room_id| parse.(kase["pr_threads_stamp"]) }
      sources = (kase["quoted_sources"] || []).map do |source|
        OpenStruct.new(updated_at: parse.(source["updated_at"]), edited_at: parse.(source["edited_at"]),
          creator: OpenStruct.new(name: source["creator"]), room: OpenStruct.new(name: source["room"]))
      end
      helper.define_singleton_method(:quoted_sources) { |_message| sources }
      key = helper.message_with_pr_cards_cache_key(message)
      [ zone_name, {
        "key" => ActiveSupport::Cache.expand_cache_key(key),
        "fragment" => ActiveSupport::Cache.expand_cache_key(helper.controller.combined_fragment_cache_key(helper.cache_fragment_name(key, digest_path: "messages/_message:0123abcd")))
      } ]
    end
  end
  kase.merge("message" => fragment_message.cache_key_with_version, "expanded" => expanded)
end
cache["quote_names_digests"] = [
  [ [ [ "b", "x" ], [ "a", "y" ] ] ], [ [ [ "Élodie \"E\" Martin", "Designers" ] ] ], [ [ [ "a\\b", "#\{c}" ], [ "new\nline", "\e\u0001\u007f" ] ] ]
].map { |(names)| [ names, Digest::SHA256.hexdigest(names.sort.inspect), names.sort.inspect ] }

# The sidebar's rooms collection (app/views/users/sidebars/show.html.erb):
# [ membership, huddle_participants_by_room_id[membership.room_id]&.map(&:id), Current.user.administrator? ]
membership = Membership.order(:id).first
cache["sidebar_membership"] = [ [ nil, false ], [ [ 3, 1 ], true ], [ [], false ] ].map do |participant_ids, administrator|
  key = [ membership, participant_ids, administrator ]
  { "membership" => membership.cache_key_with_version, "participant_ids" => participant_ids, "administrator" => administrator,
    "key" => ActiveSupport::Cache.expand_cache_key(key),
    "fragment" => ActiveSupport::Cache.expand_cache_key(view.controller.combined_fragment_cache_key(view.cache_fragment_name(key, digest_path: "users/sidebars/rooms/_shared:0123abcd"))) }
end

File.write(OUT, JSON.pretty_generate(goldens) + "\n")
