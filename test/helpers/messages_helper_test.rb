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
end
