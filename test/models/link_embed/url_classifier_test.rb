require "test_helper"

class LinkEmbed::UrlClassifierTest < ActiveSupport::TestCase
  test "extracts up to three unique URLs in order of appearance" do
    text = "a https://example.com/one b https://example.com/two c https://example.com/one " \
      "d https://example.com/three e https://example.com/four"

    assert_equal %w[
      https://example.com/one https://example.com/two https://example.com/three
    ], LinkEmbed::UrlClassifier.extract(text)
  end

  test "strips trailing sentence punctuation" do
    assert_equal [ "https://example.com/page" ],
      LinkEmbed::UrlClassifier.extract("see https://example.com/page. Next sentence")
    assert_equal [ "https://example.com/page" ],
      LinkEmbed::UrlClassifier.extract("(see https://example.com/page)")
  end

  test "keeps balanced parentheses that belong to the URL" do
    assert_equal [ "https://en.wikipedia.org/wiki/Rust_(programming_language)" ],
      LinkEmbed::UrlClassifier.extract("see https://en.wikipedia.org/wiki/Rust_(programming_language) today")
  end

  test "strips unbalanced closing parentheses and brackets" do
    assert_equal [ "https://example.com/page" ],
      LinkEmbed::UrlClassifier.extract("see https://example.com/page).")
    assert_equal [ "https://example.com/page" ],
      LinkEmbed::UrlClassifier.extract("[see https://example.com/page]")
  end

  test "skips GitHub PR, X, LinkedIn, Drive, Fizzy, and internal URLs" do
    assert_empty LinkEmbed::UrlClassifier.extract("https://github.com/rails/rails/pull/123")
    assert_empty LinkEmbed::UrlClassifier.extract("https://x.com/jack/status/20")
    assert_empty LinkEmbed::UrlClassifier.extract("https://twitter.com/jack/status/20")
    assert_empty LinkEmbed::UrlClassifier.extract("https://www.linkedin.com/feed/update/urn:li:activity:99")
    assert_empty LinkEmbed::UrlClassifier.extract("https://www.linkedin.com/posts/jane-doe_launch-99")
    assert_empty LinkEmbed::UrlClassifier.extract("https://drive.google.com/open?id=1AbcDefGhIjKlMnOpQrSt")
    assert_empty LinkEmbed::UrlClassifier.extract("https://docs.google.com/document/d/1AbcDefGhIjKlMnOpQrSt/edit")
    assert_empty LinkEmbed::UrlClassifier.extract("https://fizzy.smartdata.example.com/boards/1")
    assert_empty LinkEmbed::UrlClassifier.extract("https://smartfire.example.com/rooms/12/@34")
    assert_empty LinkEmbed::UrlClassifier.extract("https://smartfire.example.com/rooms/12/events/5")
  end

  test "special URLs do not consume the three-embed allowance" do
    text = "https://github.com/rails/rails/pull/1 https://example.com/one " \
      "https://x.com/jack/status/20 https://example.com/two"

    assert_equal %w[ https://example.com/one https://example.com/two ],
      LinkEmbed::UrlClassifier.extract(text)
  end

  test "ignores non-URL text and blank input" do
    assert_empty LinkEmbed::UrlClassifier.extract("just some text")
    assert_empty LinkEmbed::UrlClassifier.extract("ftp://example.com/file")
    assert_empty LinkEmbed::UrlClassifier.extract("")
    assert_empty LinkEmbed::UrlClassifier.extract(nil)
  end

  test "suppressed_urls finds angle-bracketed links in the source" do
    suppressed = LinkEmbed::UrlClassifier.suppressed_urls(
      "plain https://example.com/kept and <https://example.com/hidden>",
      "note with <https://example.com/noted>"
    )

    assert_equal %w[
      https://example.com/hidden https://example.com/noted
    ], suppressed
  end

  test "extract leaves suppressed URLs out" do
    suppressed = LinkEmbed::UrlClassifier.suppressed_urls("<https://example.com/hidden>")

    assert_equal [ "https://example.com/kept" ],
      LinkEmbed::UrlClassifier.extract(
        "https://example.com/kept https://example.com/hidden", suppressed: suppressed
      )
  end

  test "fizzy_url? matches Fizzy hosts only" do
    assert LinkEmbed::UrlClassifier.fizzy_url?("https://fizzy.example.com/cards/1")
    assert LinkEmbed::UrlClassifier.fizzy_url?("https://workspace.FIZZY.io/x")
    assert_not LinkEmbed::UrlClassifier.fizzy_url?("https://example.com/fizzy-drink")
    assert_not LinkEmbed::UrlClassifier.fizzy_url?("not a url")
  end
end
