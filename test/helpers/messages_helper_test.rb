require "test_helper"

class MessagesHelperTest < ActionView::TestCase
  test "message_presentation preserves sanitized Markdown structure and renders mentions" do
    message = Message.create!(
      room: rooms(:pets),
      markdown_source: "## Status\n\n```ruby\nputs :ok\n```\n\n- [x] done\n\n| Who |\n| --- |\n| @[David] |",
      client_message_id: "markdown-helper",
      creator: users(:jason)
    )

    presentation = view.message_presentation(message)

    assert_match %r{<h2>Status</h2>}, presentation
    assert_match %r{<pre><code class="language-ruby">puts :ok}, presentation
    assert_match %r{<input type="checkbox" checked="" disabled="disabled"> done}, presentation
    assert_match %r{<table>.*<div class="mention mention--user-#{users(:david).id}"}m, presentation
    assert_match %r{\sDavid\s*</div>}, presentation
    assert_match %r{<div class="mention mention--user-#{users(:david).id}" sgid="[^"]+" data-user-id="#{users(:david).id}">}, presentation
    assert_match %r{\A<div class="markdown-body" data-controller="drive-link">}, presentation
    assert_no_match /trix-content/, presentation
  end

  test "markdown presentation reads preloaded mentions without querying" do
    message = Message.create!(
      room: rooms(:pets),
      markdown_source: "Hi @[David]",
      client_message_id: "markdown-preloaded-mention",
      creator: users(:jason)
    )
    loaded = Message.with_rendering_details.find(message.id)
    Message::MentionPreloader.preload_for([ loaded ])

    presentation = nil
    assert_no_queries { presentation = view.markdown_message_presentation(loaded.body.body) }
    assert_match %r{<div class="mention mention--user-#{users(:david).id}"}, presentation
  ensure
    Current.mentioned_users_by_id = nil
  end

  test "legacy mention presentation includes a stable user id marker" do
    message = Message.create!(
      room: rooms(:pets),
      body: "<div>Hi #{mention_attachment_for(:david)}</div>",
      client_message_id: "legacy-mention-marker",
      creator: users(:jason)
    )

    presentation = view.message_presentation(message)

    assert_match %r{<div class="mention mention--user-#{users(:david).id}">}, presentation
  end

  test "message_presentation neutralizes unsafe URI schemes in links" do
    message = Message.create! room: rooms(:pets), body: '<div><a href="javascript:alert(1)">x</a></div>', client_message_id: "0015", creator: users(:jason)

    presentation = view.message_presentation(message)
    assert_no_match /javascript:/, presentation
    assert_match /<a>x<\/a>/, presentation
  end

  test "message_presentation strips event handler attributes from allowed tags" do
    message = Message.create! room: rooms(:pets), body: '<div><a href="/x" onmouseover="alert(1)">x</a></div>', client_message_id: "0015", creator: users(:jason)

    presentation = view.message_presentation(message)
    assert_no_match /onmouseover/, presentation
    assert_match /<a href="\/x">x<\/a>/, presentation
  end

  test "message_presentation renders Markdown forwards through the Markdown sanitizer" do
    create_workspace_icon(name: "acme")
    source = Message.create!(
      room: rooms(:pets),
      markdown_source: "| What |\n| --- |\n| :acme: |\n\n```ruby\nputs :ok\n```",
      client_message_id: "forward-source-markdown",
      creator: users(:jason)
    )
    forwarded = Message.create!(
      room: rooms(:pets),
      body: source.body.body.to_html,
      forwarded_from_message: source,
      forwarded_at: Time.current,
      forwarded_markdown: true,
      client_message_id: "forwarded-markdown",
      creator: users(:jason)
    )

    presentation = view.message_presentation(forwarded)

    assert_match %r{\A<div class="markdown-body" data-controller="drive-link">}, presentation
    assert_match %r{<table>.*</table>}m, presentation
    assert_match %r{<img[^>]*alt=":acme:"}, presentation
    assert_match %r{<pre><code class="language-ruby">puts :ok}, presentation
  end

  test "message_presentation keeps legacy forwards on the legacy path" do
    source = Message.create!(
      room: rooms(:pets), body: "<div>plain legacy</div>",
      client_message_id: "legacy-forward-source", creator: users(:jason)
    )
    forwarded = Message.create!(
      room: rooms(:pets),
      body: source.body.body.to_html,
      forwarded_from_message: source,
      forwarded_at: Time.current,
      client_message_id: "legacy-forward",
      creator: users(:jason)
    )

    presentation = view.message_presentation(forwarded)

    assert_no_match(/markdown-body/, presentation)
    assert_match(/plain legacy/, presentation)
  end

  test "message_presentation preserves safe links and formatting" do
    message = Message.create! room: rooms(:pets), body: '<div><a href="https://example.com">example</a> <strong>bold</strong></div>', client_message_id: "0015", creator: users(:jason)

    presentation = view.message_presentation(message)
    assert_match /<a href="https:\/\/example\.com"[^>]*>example<\/a>/, presentation
    assert_match /<strong>bold<\/strong>/, presentation
  end

  # Nokogiri leaves ">" raw inside quoted attribute values, which fooled
  # auto_link into treating the rest of the value as text and inserting an
  # anchor there. The anchor's quotes closed the attribute early, turning the
  # rest of the stored value into live markup after sanitization.
  test "legacy presentation doesn't autolink URLs inside attribute values" do
    title = "x> http://evil.test/ <img src=x onerror=alert(1)>"
    message = legacy_message(%(<p title="#{title}">hi</p>))

    rendered = Nokogiri::HTML5.fragment(view.message_presentation(message))

    assert_empty rendered.css("img, a, [onerror]")
    assert_equal title, rendered.at_css("p")["title"]
    assert_equal "hi", rendered.at_css("p").text
  end

  test "legacy presentation leaves an attribute value holding a URL byte-identical" do
    message = legacy_message(%(<span title="a>b http://example.com/x">t</span>))

    assert_equal %(<div class="trix-content">\n  <span title="a>b http://example.com/x">t</span>\n</div>\n),
      view.message_presentation(message)
  end

  test "legacy presentation doesn't autolink email addresses inside attribute values" do
    message = legacy_message(%(<span title="a>b me@example.com">t</span>))

    assert_equal %(<div class="trix-content">\n  <span title="a>b me@example.com">t</span>\n</div>\n),
      view.message_presentation(message)
  end

  test "legacy presentation still autolinks text after an attribute value holding a bracket" do
    message = legacy_message(%(<span title="a>b">t</span> http://example.com/x me@example.com))

    assert_equal %(<div class="trix-content">\n  <span title="a>b">t</span> <a target="_blank" href="http://example.com/x">http://example.com/x</a> ) +
      %(<a target="_blank" href="mailto:me@example.com">me@example.com</a>\n</div>\n),
      view.message_presentation(message)
  end

  # Scanning everything before each match made a body with many links
  # quadratic: 2,000 addresses took several times as long as without the fix.
  test "legacy presentation finds attribute values once per autolink pass, however many matches" do
    scanned = []
    view.singleton_class.prepend(Module.new do
      define_method(:quoted_attribute_values) { |html| scanned << html.bytesize; super(html) }
    end)
    message = legacy_message(%(<span title="hello">me@example.test http://example.test/ </span>) * 500)

    presentation = view.message_presentation(message)

    assert_equal 1_000, presentation.scan("<a ").size
    assert_equal 2, scanned.size, "one scan for the URL pass and one for the email pass"
  end

  private
    def legacy_message(body)
      Message.create! room: rooms(:pets), body: body, client_message_id: "legacy-autolink", creator: users(:jason)
    end
end
