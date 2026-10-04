# URL-only domain vectors from our Rails app. No database writes or outbound requests.
output = ARGV.fetch(0)
normal = ["https://EXAMPLE.com/page#section", "https://example.com/page/", "https://example.com", "https://example.com:8443/page", "http://example.com:80/page", " https://user:pass@EXAMPLE.com/path//?q=1#x ", "https://example.com/??", "not a url", "ftp://example.com/file", "", "https://", "https:/rooms/1"]
texts = [
  "a https://example.com/one b https://example.com/two c https://example.com/one d https://example.com/three e https://example.com/four",
  "see https://example.com/page. Next sentence", "(see https://example.com/page)", "see https://en.wikipedia.org/wiki/Rust_(programming_language) today", "see https://example.com/page).", "[see https://example.com/page]",
  *%w[https://github.com/rails/rails/pull/123 https://github.com/./repo/pull/12 https://github.com/acme/../pull/12 https://github.com/a/b/pulls/42/files https://x.com/jack/status/20 https://twitter.com/jack/status/20 https://mobile.x.com/i/web/status/33 https://www.linkedin.com/feed/update/urn:li:activity:99 https://www.linkedin.com/posts/jane-doe_launch-99 https://drive.google.com/open?id=1AbcDefGhIjKlMnOpQrSt https://docs.google.com/document/d/1AbcDefGhIjKlMnOpQrSt/edit https://fizzy.smartdata.example.com/boards/1 https://smartfire.example.com/rooms/12/@34 https://smartfire.example.com/rooms/12/events/5 https://example.com/fizzy-drink],
  "https://github.com/rails/rails/pull/1 https://example.com/one https://x.com/jack/status/20 https://example.com/two", "just some text", "ftp://example.com/file", "", "HTTP://EXAMPLE.com/a", "see https://example.com/page)]!"
]
sources = ["plain https://example.com/kept and <https://example.com/hidden>", "note with <https://example.com/noted>", "<https://EXAMPLE.com/a/#one> <https://example.com/a#two> <https://example.com/kept>"]
linkedin = [
  "see https://www.linkedin.com/feed/update/urn:li:activity:7234567890123456789 please", "http://linkedin.com/feed/update/urn:li:share:12345", "https://www.linkedin.com/feed/update/urn:li:ugcPost:7234567890", "https://www.linkedin.com/feed/update/urn:li:activity:abcDEF123", "https://www.linkedin.com/feed/update/urn:li:ugcPost:12ab34", "https://www.linkedin.com/feed/update/urn:li:share:",
  "https://www.linkedin.com/posts/jane-doe_launch-day-activity-7234567890-abcd", "https://www.linkedin.com/feed/update/urn:li:activity:99?commentUrn=urn%3Ali%3Acomment%3A1#frag",
  (1..5).map { |n| "https://www.linkedin.com/feed/update/urn:li:activity:#{n}" }.join(" ") + " again https://www.linkedin.com/feed/update/urn:li:activity:2?trk=public_post",
  *%w[https://www.linkedin.com/in/janedoe https://www.linkedin.com/company/acme https://www.linkedin.com/feed/ https://www.linkedin.com/feed/update/urn:li:comment:123 https://www.linkedin.com/posts/ https://example.com/feed/update/urn:li:activity:1 ftp://www.linkedin.com/posts/abc], "just some text", "", "<https://www.linkedin.com/posts/slug-55>", "(see https://www.linkedin.com/feed/update/urn:li:activity:66)", "see https://www.linkedin.com/posts/abc123. Next", "see https://www.linkedin.com/feed/update/urn:li:activity:66!", "https://www.linkedin.com/feed/update/urn:li:activity:66}", "https://www.linkedin.com/feed/update/urn:li:activity:66z"
]
html = [
  '<p>see <code>https://www.linkedin.com/posts/aaa111</code> and https://www.linkedin.com/posts/bbb222</p><pre><code class="language-text">https://www.linkedin.com/posts/ccc333</code></pre><p><a href="https://www.linkedin.com/posts/ddd444">the post</a></p>',
  '<p>see <a href="https://www.linkedin.com/posts/abc123">https://www.linkedin.com/posts/abc123</a>. Next</p>',
  '<p>hello<br>world</p><p>next</p><a href="https://example.com/a?a=1&amp;b=2">labeled</a>',
  '<p>A &amp; B</p><!--comment--><pre>gone</pre><div>x</div>'
]
fizzy = ["https://app.fizzy.do", "http://fizzy.example:51550/base", "bad URL", "", "https://workspace.example"] .flat_map do |base|
  ENV["FIZZY_API_BASE_URL"] = base
  %w[https://app.fizzy.do/account_1/cards/22 http://fizzy.example:51550/acme/cards/55#comment https://workspace.example:456/acme/cards/99 https://app.fizzy.do/../cards/2 https://app.fizzy.do/a/cards/22abc].map { |url| { base: base, url: url, card: Fizzy::CardUrl.card_url?(url) } }
end
references = [
  [ '<p>https://EXAMPLE.com/page#private https://example.com/page#other</p>', '', '' ],
  [ '<p>https://www.linkedin.com/posts/abc https://example.com/a https://example.com/b https://example.com/c https://example.com/d</p>', '', 'https://example.com/e' ],
  [ '<p>https://example.com/hidden https://example.com/kept</p>', '<https://example.com/hidden>', '' ],
  [ '<code>https://example.com/code</code><pre>https://example.com/pre</pre><p><a href="https://example.com/label#raw">label</a></p>', '', '' ],
  [ '<p>https://example.com/body</p>', '', 'https://example.com/forward#note <https://example.com/suppress>' ],
  [ '<p>https://www.linkedin.com/feed/update/urn:li:share:12?trk=room#raw</p>', '', '' ]
].map do |body, source, note|
  suppressed = LinkEmbed::UrlClassifier.suppressed_urls(source, note)
  text = [ Linkedin::PostUrl.non_code_text(body), note ].compact_blank.join("\n")
  selected = LinkEmbed::ReferenceSync.send(:raw_urls_by_normalized, text, suppressed).map { |normalized, raw| { normalized_url: normalized, url: raw } }
  { html: body, source: source, note: note, selected: selected }
end
File.write(output, JSON.pretty_generate({
  reference: ENV.fetch("PARITY_REFERENCE_SHA"), normalized: normal.map { |url| { url: url, expected: LinkEmbed.normalize_url(url) } },
  classifier: texts.map { |text| { text: text, extracted: LinkEmbed::UrlClassifier.extract(text), special: LinkEmbed::UrlClassifier.special_url?(text), github: Github::PullRequestUrl.pull_request_url?(text) } },
  suppression: { sources: sources, suppressed: LinkEmbed::UrlClassifier.suppressed_urls(*sources), extracted: LinkEmbed::UrlClassifier.extract("https://example.com/kept https://example.com/hidden https://example.com/a/", suppressed: LinkEmbed::UrlClassifier.suppressed_urls(*sources)) },
  linkedin: linkedin.map { |text| { text: text, is_post: Linkedin::PostUrl.post_url?(text), embed: Linkedin::PostUrl.embed_url_for(text), extracted: Linkedin::PostUrl.extract(text).map { |r| { url: r.url, urn: r.urn } } } },
  references: references, non_code: html.map { |input| { html: input, text: Linkedin::PostUrl.non_code_text(input) } }, fizzy: fizzy
}) + "\n")
puts "WS15e Rails URL vectors: #{normal.size} normalization, #{texts.size} classifier, #{linkedin.size} LinkedIn, #{html.size} HTML, #{fizzy.size} Fizzy, #{references.size} references"
