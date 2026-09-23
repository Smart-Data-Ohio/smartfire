require "test_helper"

class LinkEmbed::MetadataParserTest < ActiveSupport::TestCase
  test "parses OpenGraph tags" do
    result = LinkEmbed::MetadataParser.parse(<<~HTML, base_url: "https://example.com/page")
      <html><head>
        <meta property="og:title" content="Example Title">
        <meta property="og:description" content="An example description.">
        <meta property="og:site_name" content="Example">
        <meta property="og:image" content="https://example.com/image.png">
      </head></html>
    HTML

    assert_equal "Example Title", result.title
    assert_equal "An example description.", result.description
    assert_equal "Example", result.site_name
    assert_equal "https://example.com/image.png", result.image_url
  end

  test "falls back to Twitter cards, the document title, and the host" do
    result = LinkEmbed::MetadataParser.parse(<<~HTML, base_url: "https://example.com/page")
      <html><head>
        <title>Document Title</title>
        <meta name="twitter:title" content="Card Title">
        <meta name="twitter:description" content="Card description.">
        <meta name="twitter:image" content="/card.png">
      </head></html>
    HTML

    assert_equal "Card Title", result.title
    assert_equal "Card description.", result.description
    assert_equal "example.com", result.site_name
    assert_equal "https://example.com/card.png", result.image_url
  end

  test "prefers the secure image URL and resolves relative paths" do
    result = LinkEmbed::MetadataParser.parse(<<~HTML, base_url: "https://example.com/blog/page")
      <html><head>
        <meta property="og:image" content="http://cdn.example.com/plain.png">
        <meta property="og:image:secure_url" content="https://cdn.example.com/secure.png">
      </head></html>
    HTML
    assert_equal "https://cdn.example.com/secure.png", result.image_url

    relative = LinkEmbed::MetadataParser.parse(<<~HTML, base_url: "https://example.com/blog/page")
      <html><head><meta property="og:image" content="images/hero.png"></head></html>
    HTML
    assert_equal "https://example.com/blog/images/hero.png", relative.image_url
  end

  test "strips markup and truncates to the column limits" do
    result = LinkEmbed::MetadataParser.parse(<<~HTML, base_url: "https://example.com/")
      <html><head>
        <meta property="og:title" content="<img src=x onerror=alert(1)>#{"Hey! " * 100}">
        <meta property="og:description" content="<b>bold</b> words">
      </head></html>
    HTML

    assert_equal "Hey! " * 60, result.title
    assert_equal 300, result.title.length
    assert_equal "bold words", result.description
  end

  test "stores plain text so the card escapes it exactly once" do
    result = LinkEmbed::MetadataParser.parse(<<~HTML, base_url: "https://example.com/")
      <html><head>
        <meta property="og:title" content="Tom &amp; Jerry say &quot;hi&quot; &lt;3">
        <meta property="og:description" content="5 &gt; 3 &amp; 2 &lt; 4, &quot;quoted&quot;">
      </head></html>
    HTML

    assert_equal 'Tom & Jerry say "hi" <3', result.title
    assert_equal '5 > 3 & 2 < 4, "quoted"', result.description
    assert_equal "Tom &amp; Jerry say &quot;hi&quot; &lt;3", ERB::Util.html_escape(result.title)
  end

  test "entity-decoded text carries no executable markup" do
    result = LinkEmbed::MetadataParser.parse(<<~HTML, base_url: "https://example.com/")
      <html><head>
        <meta property="og:title" content="&lt;img src=x onerror=alert(1)&gt;Hi">
        <meta property="og:description" content="<a href=&quot;javascript:alert(1)&quot;>click</a>">
      </head></html>
    HTML

    assert_equal "Hi", result.title
    assert_equal "click", result.description
    assert_not_includes ERB::Util.html_escape(result.title), "<img"
    assert_not_includes ERB::Util.html_escape(result.description), "<a"
  end

  test "rejects non-web image targets" do
    result = LinkEmbed::MetadataParser.parse(<<~HTML, base_url: "https://example.com/")
      <html><head><meta property="og:image" content="javascript:alert(1)"></head></html>
    HTML

    assert_nil result.image_url
  end
end
