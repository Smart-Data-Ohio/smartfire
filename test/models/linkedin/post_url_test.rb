require "test_helper"

class Linkedin::PostUrlTest < ActiveSupport::TestCase
  test "extracts a feed/update activity URL with its URN" do
    assert_equal [
      Linkedin::PostUrl::Reference.new(
        "https://www.linkedin.com/feed/update/urn:li:activity:7234567890123456789",
        "urn:li:activity:7234567890123456789"
      )
    ], Linkedin::PostUrl.extract("see https://www.linkedin.com/feed/update/urn:li:activity:7234567890123456789 please")
  end

  test "extracts share and ugcPost URNs with http and bare hosts" do
    assert_equal "urn:li:share:12345",
      Linkedin::PostUrl.extract("http://linkedin.com/feed/update/urn:li:share:12345").first.urn
    assert_equal "urn:li:ugcPost:abcDEF123",
      Linkedin::PostUrl.extract("https://www.linkedin.com/feed/update/urn:li:ugcPost:abcDEF123").first.urn
  end

  test "extracts posts URLs without a URN" do
    reference = Linkedin::PostUrl.extract("https://www.linkedin.com/posts/jane-doe_launch-day-activity-7234567890-abcd").first

    assert_equal "https://www.linkedin.com/posts/jane-doe_launch-day-activity-7234567890-abcd", reference.url
    assert_nil reference.urn
  end

  test "ignores trailing paths, query strings, and fragments" do
    references = Linkedin::PostUrl.extract(
      "https://www.linkedin.com/feed/update/urn:li:activity:99?commentUrn=urn%3Ali%3Acomment%3A1#frag"
    )

    assert_equal 1, references.size
    assert_equal "urn:li:activity:99", references.first.urn
  end

  test "dedupes repeats and keeps at most three in order of appearance" do
    text = (1..5).map { |n| "https://www.linkedin.com/feed/update/urn:li:activity:#{n}" }.join(" ") +
      " again https://www.linkedin.com/feed/update/urn:li:activity:2?trk=public_post"

    assert_equal %w[
      urn:li:activity:1 urn:li:activity:2 urn:li:activity:3
    ], Linkedin::PostUrl.extract(text).map(&:urn)
  end

  test "ignores non-post URLs" do
    assert_empty Linkedin::PostUrl.extract("https://www.linkedin.com/in/janedoe")
    assert_empty Linkedin::PostUrl.extract("https://www.linkedin.com/company/acme")
    assert_empty Linkedin::PostUrl.extract("https://www.linkedin.com/feed/")
    assert_empty Linkedin::PostUrl.extract("https://www.linkedin.com/feed/update/urn:li:comment:123")
    assert_empty Linkedin::PostUrl.extract("https://www.linkedin.com/posts/")
    assert_empty Linkedin::PostUrl.extract("https://example.com/feed/update/urn:li:activity:1")
    assert_empty Linkedin::PostUrl.extract("ftp://www.linkedin.com/posts/abc")
    assert_empty Linkedin::PostUrl.extract("just some text")
    assert_empty Linkedin::PostUrl.extract(nil)
  end

  test "non_code_text drops code spans and fenced blocks but keeps prose and labeled links" do
    html = <<~HTML
      <p>see <code>https://www.linkedin.com/posts/aaa111</code> and https://www.linkedin.com/posts/bbb222</p>
      <pre><code class="language-text">https://www.linkedin.com/posts/ccc333</code></pre>
      <p><a href="https://www.linkedin.com/posts/ddd444">the post</a></p>
    HTML

    text = Linkedin::PostUrl.non_code_text(html)

    assert_not_includes text, "aaa111"
    assert_not_includes text, "ccc333"
    assert_includes text, "bbb222"
    assert_includes text, "ddd444"
  end

  test "embed_url_for builds the official player URL only for URN links" do
    assert_equal "https://www.linkedin.com/embed/feed/update/urn:li:share:12345",
      Linkedin::PostUrl.embed_url_for("https://www.linkedin.com/feed/update/urn:li:share:12345")
    assert_nil Linkedin::PostUrl.embed_url_for("https://www.linkedin.com/posts/jane-doe_launch-day-123")
    assert_nil Linkedin::PostUrl.embed_url_for("https://example.com/")
  end
end
