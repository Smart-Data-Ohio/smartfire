require "test_helper"

class Slack::MarkdownConverterTest < ActiveSupport::TestCase
  USERS = { "U001" => "Jane Doe", "U002" => "Kevin" }.freeze

  def convert(text, users: USERS, **message_fields)
    Slack::MarkdownConverter.convert({ "text" => text }.merge(message_fields.stringify_keys), users:)
  end

  test "unescapes Slack entities" do
    assert_equal "fish & chips <tag>", convert("fish &amp; chips &lt;tag&gt;").markdown
  end

  test "user mentions become Smartfire mention tokens" do
    assert_equal "hi @[Jane Doe]!", convert("hi <@U001>!").markdown
    assert_equal "hi @[Jane Doe]!", convert("hi <@U001|jane>!").markdown
  end

  test "unknown users fall back to the label or id" do
    assert_equal "hi @ghost!", convert("hi <@U999|ghost>!").markdown
    assert_equal "hi @U999!", convert("hi <@U999>!").markdown
  end

  test "channel references become hashes" do
    assert_equal "see #secret", convert("see <#CPRIV|secret>").markdown
    assert_equal "see #CPRIV", convert("see <#CPRIV>").markdown
  end

  test "broadcast mentions stay literal text" do
    assert_equal "@here standup", convert("<!here> standup").markdown
    assert_equal "@channel news", convert("<!channel> news").markdown
    assert_equal "@everyone hi", convert("<!everyone> hi").markdown
  end

  test "broadcast mentions with labels stay literal text" do
    assert_equal "@here standup", convert("<!here|here> standup").markdown
    assert_equal "@here standup", convert("<!here|@here> standup").markdown
    assert_equal "@channel news", convert("<!channel|channel> news").markdown
    assert_equal "@everyone hi", convert("<!everyone|everyone> hi").markdown
  end

  test "subteam mentions become handles and dates become fallbacks" do
    assert_equal "ping @engs", convert("ping <!subteam^S123|@engs>").markdown
    assert_equal "due Feb 1", convert("due <!date^1700000000^{date_short}|Feb 1>").markdown
  end

  test "date tokens keep their fallback with or without a link part" do
    assert_equal "due Feb 1", convert("due <!date^1700000000^{date_short}^https://example.com|Feb 1>").markdown
    assert_equal "due ", convert("due <!date^1700000000^{date_short}>").markdown
  end

  # Regression guard, not a proof of the fix: on Ruby 3.4 the regex
  # engine memoizes backtracking, so this input converts in linear time
  # even with the old nested-repetition pattern (measured ~7ms at 50k
  # repeats for the old pattern, ~1ms for the new one, and neither trips
  # a Regexp.timeout). A static "no nested quantifier" check cannot
  # discriminate either: the new pattern still quantifies a group that
  # holds a quantified class. The guard stays to catch a future pattern
  # (or engine) that turns crafted date tokens super-linear again.
  test "a crafted date token converts in linear time" do
    started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    result = convert("<!date^" + "!^" * 50_000)
    elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - started

    assert_operator elapsed, :<, 0.5, "date token conversion took #{elapsed.round(3)}s"
    assert result.markdown.start_with?("<!date^!^")
  end

  test "links convert to markdown and bare urls pass through" do
    assert_equal "see [docs](https://example.com)", convert("see <https://example.com|docs>").markdown
    assert_equal "see https://example.com", convert("see <https://example.com>").markdown
  end

  test "mailto links convert" do
    assert_equal "write [us](mailto:team@example.com)", convert("write <mailto:team@example.com|us>").markdown
    assert_equal "write team@example.com", convert("write <mailto:team@example.com>").markdown
  end

  test "emphasis converts to markdown" do
    assert_equal "a **bold** word", convert("a *bold* word").markdown
    assert_equal "an *italic* word", convert("an _italic_ word").markdown
    assert_equal "a ~~struck~~ word", convert("a ~struck~ word").markdown
  end

  test "emphasis leaves snake_case and code untouched" do
    assert_equal "var snake_case_name here", convert("var snake_case_name here").markdown
    assert_equal "`*literal*` and **bold**", convert("`*literal*` and *bold*").markdown
    assert_equal "```\n*block*\n```", convert("```\n*block*\n```").markdown
  end

  test "emphasis leaves urls alone" do
    result = convert("see <https://example.com/a*b_c|a*b_c>").markdown
    assert_equal "see [a*b_c](https://example.com/a*b_c)", result
  end

  test "emphasis leaves bare urls and angle-bracket links alone" do
    assert_equal "see https://example.com/*path*/x_y",
      convert("see <https://example.com/*path*/x_y>").markdown
    assert_equal "see https://example.com/*path*/x_y and **bold**",
      convert("see https://example.com/*path*/x_y and *bold*").markdown
  end

  test "code on the fence's first line stays code" do
    # Slack has no language tags: the first line is code, so it must not
    # sit on the fence line where Markdown would read it as one.
    assert_equal "```\nconst x = 1\nputs x\n```",
      convert("```const x = 1\nputs x\n```").markdown
    assert_equal "```\ncode\n```", convert("```code```").markdown
    assert_equal "```\n*block*\n```", convert("```\n*block*\n```").markdown
  end

  test "bullets become dashes" do
    assert_equal "- item", convert("• item").markdown
  end

  test "emoji shortcodes are kept and skin tones stripped" do
    assert_equal "nice :wave: done", convert("nice :wave::skin-tone-2: done").markdown
  end

  test "files append link lines" do
    result = convert("see this", files: [
      { "name" => "spec.pdf", "permalink" => "https://files.example/s.pdf" },
      { "title" => "shot", "permalink_public" => "https://files.example/shot" }
    ])

    assert_equal 2, result.files_linked
    assert_includes result.markdown, "📎 [spec.pdf](https://files.example/s.pdf)"
    assert_includes result.markdown, "📎 [shot](https://files.example/shot)"
  end

  test "files without any url are skipped" do
    result = convert("see this", files: [ { "name" => "x" } ])

    assert_equal 0, result.files_linked
    assert_equal "see this", result.markdown
  end

  test "attachments quote in when the text is empty" do
    result = convert("", attachments: [
      { "pretext" => "Build", "text" => "All green", "fallback" => "Build passed" }
    ])

    assert_equal "> Build\n> All green\n> Build passed", result.markdown
  end

  test "bot messages quote attachments even with text" do
    result = convert("Build *passed*", subtype: "bot_message", bot_id: "B1",
      attachments: [ { "text" => "All green" } ])

    assert_includes result.markdown, "Build **passed**"
    assert_includes result.markdown, "> All green"
  end

  test "human messages with text skip attachments" do
    result = convert("look", attachments: [ { "text" => "unfurl" } ])

    assert_equal "look", result.markdown
  end

  test "me messages become italic" do
    assert_equal "*waves hello*", convert("waves hello", subtype: "me_message").markdown
  end

  test "long messages truncate at the source limit with a flag" do
    result = convert("x" * (Message::Markdown::SOURCE_LIMIT + 100))

    assert result.truncated
    assert_equal Message::Markdown::SOURCE_LIMIT, result.markdown.length
    assert result.markdown.end_with?("… (import truncated)")
  end

  test "short messages are not flagged" do
    refute convert("hello").truncated
  end

  test "rendering: mentions resolve to attachments through a real save" do
    room = rooms(:hq)
    jane = User.create!(name: "Jane Doe", email_address: "conv-jane@example.com")

    begin
      room.memberships.grant_to([ jane ])
      message = room.messages.create!(creator: users(:david),
        markdown_source: convert("hi <@U001>!").markdown)

      assert_includes message.body.body.to_html, "application/vnd.campfire.mention"
      assert_equal [ jane ], message.mentionees.to_a
    ensure
      jane.destroy!
      message.destroy!
    end
  end

  test "rendering: broadcast mentions create no attachments or notifications" do
    room = rooms(:hq)
    message = room.messages.create!(creator: users(:david),
      markdown_source: convert("<!here> standup").markdown)

    assert_includes message.body.body.to_html, "@here"
    assert_empty message.mentionees.to_a
  ensure
    message&.destroy!
  end

  test "rendering: fenced first-line code and bare urls render as intended" do
    room = rooms(:hq)
    source = convert("```const x = 1\nputs x\n```\n\nsee https://example.com/docs").markdown
    message = room.messages.create!(creator: users(:david), markdown_source: source)
    html = message.body.body.to_html

    assert_includes html, "const x = 1"
    assert_includes html, 'href="https://example.com/docs"'
  ensure
    message&.destroy!
  end

  test "rendering: emphasis, links, bullets and quotes render as markdown" do
    room = rooms(:hq)
    source = convert("*bold* _it_ ~gone~\n\n• item\n\nsee <https://example.com|docs>").markdown
    message = room.messages.create!(creator: users(:david), markdown_source: source)
    html = message.body.body.to_html

    assert_includes html, "<strong>bold</strong>"
    assert_includes html, "<em>it</em>"
    assert_includes html, "<del>gone</del>"
    assert_includes html, "<li>item</li>"
    assert_includes html, 'href="https://example.com"'
  ensure
    message&.destroy!
  end
end
