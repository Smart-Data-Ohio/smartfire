require_relative "message_states"

# Review regressions and the cross-cutting helper audit. Every expectation is actual Rails HTML.
def generate_review_goldens(goldens)
  controller = GoldenController.new
  controller.set_request! ActionDispatch::Request.new(Rack::MockRequest.env_for("http://campfire.test/", "rack.session" => {}))
  controller.set_response! ActionDispatch::Response.new
  view = controller.view_context
  out = goldens["helpers"]["review"] = {}
  out["sidebar_empty"] = view.sidebar_turbo_frame_tag { "" }
  out["sidebar_src"] = view.sidebar_turbo_frame_tag(src: "/users/me/sidebar?x=1&y=2") { "<b>Body</b>".html_safe }
  out["involvements"] = [Rooms::Open, Rooms::Closed, Rooms::Direct].flat_map do |klass|
    room = klass.new(id: 1, name: "HQ")
    view.involvement_levels_for(room).map do |level|
      { direct: room.direct?, param_key: room.model_name.param_key, level: level,
        next: view.send(:next_involvement_for, room, involvement: level),
        label: view.short_involvement_label(level), description: view.involvement_description(level),
        html: view.button_to_change_involvement(room, level) }
    end
  end
  out["direct_buttons"] = ["Jane", "Jane <&> \"Doe\""].map do |name|
    { name: name, html: view.button_to_direct_room_with(User.new(id: 2, name: name)) }
  end
  goldens["layouts"]["frame_authenticated"] = render_with(inline: "Body", layout: "turbo_rails/frame")
  goldens["layouts"]["frame_head"] = render_with(inline: '<% content_for :head do %><meta name="extra" content="yes"><% end %>Body', layout: "turbo_rails/frame")

  # PR #151's one authorized correction, applied only to this throwaway oracle render.
  # The pinned file raises MissingAssetError; do not silently pretend the pin already moved.
  controller.request.env["HTTP_USER_AGENT"] = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36 Edge/124.0.0.0"
  controller.remove_instance_variable(:@platform) if controller.instance_variable_defined?(:@platform)
  corrected_edge_source = File.read(Rails.root.join("app/views/pwa/_install_instructions.html.erb")).sub('"install-edge.svg"', '"external/install-edge.svg"')
  out["edge_install_corrected"] = view.render(inline: corrected_edge_source)
  controller.request.env["HTTP_USER_AGENT"] = USER_AGENT
  controller.remove_instance_variable(:@platform)

  # Links, controls and profile triggers: compare complete tags (including data-action and aria).
  out["link_room"] = view.link_to_room(Rooms::Open.new(id: 1), class: "btn", data: { room_id: 9, action: "click->test#open" }) { "<b>Room</b>".html_safe }
  out["link_back"] = view.link_back_to("/rooms/1")
  out["link_back_referrers"] = [nil, "http://campfire.test/", "http://campfire.test/rooms/1"].map do |referrer|
    controller.request.env["HTTP_REFERER"] = referrer
    view.link_back
  end
  out["link_back_visited"] = [nil, Rooms::Open.new(id: 1)].map do |room|
    view.define_singleton_method(:last_room_visited) { room }
    view.link_back_to_last_room_visited
  end
  out["clipboard"] = view.button_to_copy_to_clipboard("a <&> b") { "Copy" }
  out["qr"] = view.link_to_zoom_qr_code("https://campfire.test/?a=1&b=2") { "QR" }
  out["web_share"] = view.web_share_session_button("/session", "Title <&>", "Text") { "Share" }
  out["filter_menu"] = view.user_filter_menu_tag { "<b>User</b>".html_safe }
  out["filter_search"] = view.user_filter_search_tag
  out["profile_submit"] = view.profile_form_submit_button
  out["profile_trigger"] = [false, true].map { |keyboard| view.tag.span("Jane", data: view.profile_card_trigger(User.new(id: 2), keyboard: keyboard)) }
  out["turbo_frame"] = view.turbo_frame_tag("custom", src: "/frame?a=1&b=2", target: "_top", data: { action: "turbo:frame-load->test#loaded" }) { "Body" }
  out["turbo_reload"] = view.turbo_page_requires_reload_tag
  out["cable_meta"] = view.script_aware_action_cable_meta_tag
  out["current_user_nil"] = view.current_user_meta_tags.to_s
  Current.user = User.new(id: 2, name: "Jane <&>")
  out["current_user"] = view.current_user_meta_tags
  Current.reset
  out["title_default"] = view.page_title_tag
  view.instance_variable_set(:@page_title, "Title <&>")
  out["title_escaped"] = view.page_title_tag
  out["version_badge"] = view.version_badge
  account = Account.first
  Account.define_singleton_method(:first) { account }
  out["custom_styles"] = [nil, "", ".card { color: red; }"].map do |styles|
    account.custom_styles = styles
    view.custom_styles_tag.to_s
  end
  Account.singleton_class.remove_method(:first)
  out["settings"] = [nil, "light", "dark", "system", "invalid"].product([nil, "smaller", "small", "default", "large", "larger", "invalid"]).map do |theme, size|
    Current.user = User.new(theme: theme, text_size: size)
    { theme: theme, text_size: size, user_theme: view.user_theme, user_text_size: view.user_text_size,
      html: view.tag.meta(name: "color-scheme", content: view.theme_color_scheme_meta_content) }
  end
  out["zones_meta"] = [[nil, false], [nil, true], ["Eastern Time (US & Canada)", false], ["", false], ["", true]].map do |zone, explicit|
    Current.user = User.new(time_zone: zone, time_zone_explicit: explicit)
    { zone: zone, explicit: explicit, html: view.tag.meta(name: "time-zone", content: view.current_user_time_zone_meta_content) }
  end
  out["body_classes"] = [false, true].product([false, true], [nil, "custom"]).map do |admin, logo, body|
    Current.user = User.new(role: admin ? "administrator" : "member")
    account = Account.first
    account.logo.define_singleton_method(:attached?) { logo }
    Account.define_singleton_method(:first) { account }
    view.instance_variable_set(:@body_class, body)
    { admin: admin, logo: logo, body: body, result: view.body_classes }
  ensure
    Account.singleton_class.remove_method(:first)
  end
  Current.reset
  out["account_logo"] = [nil, "small"].map { |style| view.account_logo_tag(style: style) }
  out["avatars"] = [ ["github", false], ["tada", false], ["no_such_icon", false], ["github", true] ].map do |icon, uploaded|
    bot = User.find_by!(name: "Bender Bot")
    bot.icon_name = icon
    bot.avatar.define_singleton_method(:attached?) { uploaded }
    { icon: icon, uploaded: uploaded, id: bot.id, title: bot.title,
      avatar_path: view.fresh_user_avatar_path(bot),
      html: view.avatar_tag(bot, size: 32, class: "extra", loading: "lazy") }
  end
  out["icons"] = ["github", "tada", "no_such_icon"].map do |name|
    { name: name, html: view.icon_avatar_tag(name, size: 24, class: "extra", style: "color: red", aria: { label: "Override" }).to_s }
  end
  out["textarea"] = view.form_with(url: "/x", scope: "room") { |f| f.textarea(:description, value: "\nhi") }
  out["search_label"] = view.form_with(url: "/x", scope: "room", method: :get) { |f| f.label(:q, "Find <&>", class: "label") + f.search_field(:q, value: "a <&>", data: { action: "input->search#query" }) }
  out["button_form"] = view.button_to("/x", method: :delete, form: { data: { action: "submit->test#send" } }, aria: { label: "Delete" }) { "Delete" }
  out["tokenless_form"] = view.form_with(url: "/x", authenticity_token: false) { "Body" }
  out["radio"] = view.radio_button_tag("mode", "on", true, id: "radio-id", data: { action: "change->test#choose" }, aria: { label: "On" })
  out["datetime"] = view.datetime_local_field_tag("dt", "2026-01-01T09:00", data: { action: "change->test#choose" }, aria: { label: "Date" })

  # Real Rails message-cache output and facts for the reviewer's two-viewer test.
  Rails.cache = ActiveSupport::Cache::MemoryStore.new
  GoldenController.perform_caching = true
  message = Message.with_rendering_details.find_by!(client_message_id: "0001")
  out["message_html"] = %w[david@37signals.com jz@37signals.com].to_h do |email|
    html = render_with(user: user(email), partial: "messages/message", locals: { message: message })
    raise "Rails cached message has a token or failed" if html.include?("authenticity_token") || html.include?("unrenderable")
    [email, html]
  end
  out["message"] = {
    id: message.id, client_message_id: message.client_message_id, room_id: message.room_id, room_name: message.room.name,
    creator: { id: message.creator.id, name: message.creator.name, title: message.creator.title, avatar_url: view.fresh_user_avatar_path(message.creator) },
    created_at: message.created_at.iso8601(6), updated_at: message.updated_at.iso8601(6), all_emoji: false,
    content: { type: "text", html: view.message_presentation(message) },
    boosts: message.ordered_boosts.map do |boost|
      { id: boost.id, updated_at: boost.updated_at.iso8601(6), message_id: boost.message_id,
        content: boost.content, all_emoji: boost.content.all_emoji?,
        booster: { id: boost.booster.id, name: boost.booster.name, title: boost.booster.title,
          avatar_url: view.fresh_user_avatar_path(boost.booster) } }
    end
  }
  out["reaction_probes"] = ["👍", "👍🏽", "👨‍👩‍👧‍👦", "🇺🇸", "1️⃣", "#️⃣", "❤️", "🎉", "🎉🎉", "1", "#", "©", "Hello", ":github:", ":tada:", ":no_such_icon:", " 👍 ", "", *message_icon_registry[:shortcodes].keys.map { |name| ":#{name}:" }].uniq.map do |content|
    { content: content, reaction: Boost.reaction?(content), title: view.reaction_title(content),
      body: view.reaction_chip_body(content), legacy_body: view.boost_content_html(Boost.new(content: content)) }
  end
  generate_message_states(goldens, view)
  puts "Rails WS6 review: 13 notification states, #{out.size} helper groups, 2 frame layouts, 2 token-free message viewers"
ensure
  Current.reset
end
