# The Rails reference is the oracle. No HTML expected values are handwritten.
require "json"
require "yaml"
require "digest"
# Error logging is observable: invalid-encoding exception messages can make the helper
# rescue raise. Preserve the production logger behavior rather than suppressing it.
Rails.logger.level = :error
ActiveRecord::Base.logger = nil
TIME = Time.utc(2024, 1, 2, 3, 4, 5)

def user(name, active: true)
  id = User.insert!({ name:, email_address: "ws5-#{User.count}@example.test", status: active ? 0 : 1, role: 0, created_at: TIME, updated_at: TIME }).rows.first.first
  User.find(id)
end
users = [user("David"), user("Duplicate"), user("Duplicate"), user("Inactive", active: false), user("Nonmember"), user("Seán O'Brien 🎉"), user('A <b>& "B"'), user("Star *_[Name]")]
%w[openai acme smile].each do |icon|
  bot = user("Bot #{icon}")
  bot.update_columns(role: 2, icon_name: icon)
  users << bot
end
Room.insert!({ name: "WS5", type: "Rooms::Open", creator_id: users.first.id, created_at: TIME, updated_at: TIME })
room = Room.first
users.reject { |u| u.name == "Nonmember" }.each { |u| Membership.insert!({ room_id: room.id, user_id: u.id, created_at: TIME, updated_at: TIME }) }
WorkspaceIcon.insert!({ name: "acme", title: 'Acme & "custom"', creator_id: users.first.id, created_at: TIME, updated_at: TIME })
Icons.expire_custom_cache!

request = ActionDispatch::Request.new(Rack::MockRequest.env_for("http://once.campfire.test/"))
controller = MessagesController.new
controller.set_request!(request)
controller.set_response!(ActionDispatch::Response.new)
view = controller.view_context

cases = []
add = ->(name, source) { cases << { name:, mode: "markdown", source: } }
JSON.parse(File.read("/tools/specs/commonmark-0.31.2.json")).each { |c| add.call("CommonMark #{c['example']} #{c['section']}", c.fetch("markdown")) }
# cmark-gfm's test/spec.txt is the official executable spec. Keep every example.
spec = File.read("/tools/specs/gfm-0.29.0.gfm.13.txt")
fence = "`" * 32
spec.scan(/^#{fence} example[^\n]*\n(.*?)^\.\n(.*?)^#{fence}$/m).each_with_index { |(markdown, _), i| add.call("GFM #{i + 1}", markdown) }
[
  "Hi @[David] @[Duplicate] @[Inactive] @[Nonmember] @[Unknown] @[Seán O'Brien 🎉]", "@[A <b>& \"B\"]", "@[Star *_[Name]]",
  "`@[David] :smile:`\n\n```c++\n@[David] :openai:\n```", '[@[David]](https://example.com/@[David] "Ping @[David]")',
  '\@[David] \\@[David] @[David]', "@[David]\n\n[David]: /rooms/1", "@[ ] @[x\ny] @[x[y]] @[David]@[David]",
  "| Person |\n| --- |\n| @[David] |", "- [x] done\n- [ ] pending\n\n3. third", "::: :acme: :openai: :chatgpt: :smile: :unknown:",
  ":openai:word word:openai: ::openai: :OPENAI: :+1: :smile::smile: é:smile: :smile:é", ":openai::acme: :acme::openai:",
  "[jump](/rooms/1/@2) [file](/rails/active_storage/blobs/x) [out](//evil.test) [bad](javascript:alert(1))",
  "```c#\nx\n```\n\n```ruby{evil}\nx\n```", "x" * 50_000, "😀" * 50_000
].each_with_index { |s, i| add.call("handwritten #{i}", s) }
# Exercise every gemoji 4.1.0 alias, including ones intentionally outside SHORTCODE_PATTERN.
emoji_aliases = {}
::Emoji.all.each { |e| e.aliases.each { |aka| emoji_aliases[aka] ||= e.raw } }
emoji_aliases.each_key { |aka| add.call("gemoji #{aka}", ":#{aka}:") }
Icons.brands.each { |b| [b.name, *b.aliases].each { |aka| add.call("brand #{aka}", ":#{aka}:") } }
add.call("bot avatar mentions", "@[Bot openai] @[Bot acme] @[Bot smile]")
["\u00a0", "\u1680", "\u2000", "\u2003", "\u202f", "\u205f", "\u3000"].each do |space|
  add.call("review trailing mention #{space.ord.to_s(16)}", "@[David]#{space}")
  add.call("review trailing shortcode #{space.ord.to_s(16)}", ":smile:#{space}")
end


schemes = ["javascript:alert(1)", "jav&#x09;ascript:x", "&#106;avascript:x", "data:text/html,x", "vbscript:x", "\njavascript:x", "https://example.com", "/rooms/1"]
tags = %w[script style svg math iframe object embed form button input img a table pre code span div]
random = Random.new(20260929)
400.times do |i|
  tag = tags.sample(random: random)
  url = schemes.sample(random: random)
  html = %(<#{tag} href="#{url}" src="#{url}" onerror="alert(1)" onclick="alert(1)" style="background:url(javascript:x)" class="evil">&lt;img src=x onerror=alert(1)&gt; @[David] :acme:</#{tag}>)
  add.call("XSS markdown #{i}", html + "\n\n[x](#{url})\n\n![x](#{url})")
  cases << { name: "XSS presentation #{i}", mode: "sanitize", body: html }
end
avatars = ["/users/1/avatar", "/users/1/avatar?v=x", "https://assets.example.test/users/1/avatar", "https://assets12.example.test/users/1/avatar", "//assets.example.test/users/1/avatar", "//evil.test/users/1/avatar", "/\\evil.test/users/1/avatar", "data:/users/1/avatar", "/users/1/avatar/extra", "/users/x/avatar#frag", "https://assets.example.test:443/users/1/avatar", "/users/%ZZ/avatar", "/users/é/avatar"]
[nil, "assets.example.test/", "https://assets.example.test/", "//assets%d.example.test"].each do |host|
  avatars.each { |src| cases << { name: "avatar #{host.inspect} #{src}", mode: "sanitize", asset_host: host, body: %(<img src="#{src}" aria-hidden="true" onerror="x">) } }
end
[":openai:", ":chatgpt:", ":acme:", ":removed:", ":smile:", "junk"].each { |alt| cases << { name: "icon spoof #{alt}", mode: "sanitize", body: %(<img class="icon icon--custom" src="https://evil.test/tracker" alt="#{alt}">) } }
["/rooms/1", "//evil.test", "/\\evil.test", "/\t/evil.test", "/\n/evil.test", "/rails/active_storage/blobs/x"].each { |href| cases << { name: "in-app #{href.inspect}", mode: "sanitize", body: %(<a href="#{href}" target="_blank" rel="nofollow" data-turbo-prefetch="true">jump</a>) } }

legacy = ["<div>hello <strong>bold</strong><br>next</div>", "<table><tr><td>hidden</td></tr></table><s>hidden</s><u>hidden</u><mark>hidden</mark>", "<h2>Title</h2><blockquote><p>quote</p></blockquote><ol start=3><li>one<ul><li>child</li></ul></li><li>two</li></ol>", "<pre><code class='language-ruby'>a```b\n</code></pre><p><code>a`b</code></p>", "<p>\\_*[]<a href='https://example.com/a&gt;b'>link</a></p>"]
attachment = ActionText::Attachment.from_attachable(users.first, content_type: Message::Markdown::MENTION_CONTENT_TYPE).to_html
legacy << "<div>Hey #{attachment}</div>"
legacy << "<div>" + users.select(&:bot?).map { |u| ActionText::Attachment.from_attachable(u, content_type: Message::Markdown::MENTION_CONTENT_TYPE).to_html }.join(" ") + "</div>"
["/rooms/1", "https://once.campfire.test/rooms/1"].each do |href|
  cases << { name: "review solo raw href #{href}", mode: "legacy", body: %(<div>#{href}</div><action-text-attachment content-type="application/vnd.actiontext.opengraph-embed" href="#{href}" filename="Room"></action-text-attachment>) }
end
legacy << '<div>https://example.com/</div><action-text-attachment content-type="application/vnd.actiontext.opengraph-embed" href="https://example.com/" filename="Example"></action-text-attachment>'
legacy << '<div>https://x.com/dhh/status/1?q=x<action-text-attachment content-type="application/vnd.actiontext.opengraph-embed" href="https://twitter.com/dhh/status/1" url="https://pbs.twimg.com/profile_images/x.jpg" filename="Tweet"></action-text-attachment></div>'
legacy << '<action-text-attachment content-type="image/png" url="https://example.com/a.png" caption="Image"></action-text-attachment>'
legacy << '<figure data-trix-attachment="{&quot;sgid&quot;:&quot;' + users.first.attachable_sgid + '&quot;,&quot;contentType&quot;:&quot;application/vnd.campfire.mention&quot;}"></figure>'
YAML.load(ERB.new(File.read(Rails.root.join("test/fixtures/action_text/rich_texts.yml"))).result).each { |name, row| cases << { name: "fixture #{name}", mode: "legacy", body: row.fetch("body") } }
legacy.each_with_index { |body, i| cases << { name: "legacy #{i}", mode: "legacy", body: } }
preloaded_sgids = {}
[users.first.attachable_sgid.split("--").first + "--tampered", users.first.to_sgid(expires_in: nil, for: "transfer").to_s].each_with_index do |sgid, i|
  preloaded_sgids[sgid] = users.first.id
  cases << { name: "preloaded user #{i}", mode: "legacy", preload: true, body: %(<div>Hi <action-text-attachment sgid="#{sgid}" content-type="application/vnd.campfire.mention"></action-text-attachment></div>) }
end


results = Current.set(request:) do
  ActionText::Content.with_renderer(controller) do
    cases.each_with_index.map do |c, i|
      Rails.configuration.action_controller.asset_host = c[:asset_host]
      Current.mentioned_users_by_id = c[:preload] ? { users.first.id => users.first } : {}
      if c[:mode] == "sanitize"
        c.merge(output: Message::Markdown.sanitize_presentation(c[:body]))
      else
        html = c[:mode] == "markdown" ? Message::Markdown.render(c[:source], room:) : c[:body]
        Message.insert!({ room_id: room.id, creator_id: users.first.id, client_message_id: "ws5-#{i}", markdown_source: c[:source], created_at: TIME, updated_at: TIME })
        message = Message.find_by!(client_message_id: "ws5-#{i}")
        ActionText::RichText.insert!({ record_type: "Message", record_id: message.id, name: "body", body: html, created_at: TIME, updated_at: TIME })
        message.reload
        result = c.merge(rendered: html, presentation: view.message_presentation(message).to_s, editable: Message::LegacyMarkdown.render(message.body.body.to_html), plain_text: c[:mode] == "markdown" ? Message::Markdown.plain_text(message.body.body) : message.plain_text_body)
        result[:mentioned] = message.send(:mentioned_users).map(&:id) if c[:preload]
        if c[:mode] == "markdown"
          message.update_columns(markdown_source: nil, forwarded_markdown: true)
          result[:forwarded_presentation] = view.message_presentation(message.reload).to_s
        end
        result
      end
    end
  end
end
Rails.configuration.action_controller.asset_host = nil
icon_records = Icons.brands.flat_map { |b| [b.name, *b.aliases].map { |aka| [aka, { kind: b.kind, name: b.name, title: b.title, url: Icons.image_url_for(b) }] } }.to_h
Icons.custom_icons.each { |b| icon_records[b.name] = { kind: b.kind, name: b.name, title: b.title, url: Icons.image_url_for(b) } }
images = results.flat_map { |c| c[:body].to_s.scan(/url="(https:[^"]+)"/).flatten }.uniq.to_h { |url| [url, Embeds::ImageProxy.signed_path(url)] }
users_json = users.map { |u| { id: u.id, name: u.name, active: u.active?, member: u.name != "Nonmember", title: u.title, attachable_sgid: u.attachable_sgid, user_path: view.user_path(u), avatar_path: view.fresh_user_avatar_path(u), mention_partial: view.render(partial: "users/mention", locals: { user: u }), avatar_html: view.avatar_image_tag(u, size: 48, aria: { hidden: true }) } }
manifest = File.readlines("/tools/reference-files.txt", chomp: true).to_h { |p| [p, Digest::SHA256.file(Rails.root.join(p)).hexdigest] }
FileUtils.mkdir_p("/output/data")
File.write("/output/data/gemoji-4.1.0.json", JSON.generate(emoji_aliases))
File.write("/output/tests/markdown/expected.json", JSON.generate({ reference_sha256: manifest, preloaded_sgids:, commonmarker: Gem.loaded_specs.fetch("commonmarker").version.to_s, users: users_json, icons: icon_records, embed_images: images, cases: results }))
puts "Generated #{results.size} cases: #{results.group_by { |c| c[:mode] }.transform_values(&:size)}; #{emoji_aliases.size} gemoji aliases"

parser_inputs = ["<a><b><ins><sup><action-text-attachment><div></a></div>hello"]
File.write("/output/tests/markdown/parser.json", JSON.generate(parser_inputs.map { |input| { input:, output: Nokogiri::HTML5.fragment(input).to_html } }))
limits = ["x" * 50_000, "x" * 50_001, "😀" * 50_000, "😀" * 50_001].map do |source|
  message = Message.new(room:, creator: users.first, markdown_source: source, client_message_id: "limit-check")
  message.valid?
  { character: source[0], count: source.length, too_long: message.errors.of_kind?(:markdown_source, :too_long) }
end
File.write("/output/tests/markdown/limits.json", JSON.generate(limits))
