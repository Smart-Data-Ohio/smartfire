require "test_helper"

class LinkEmbedTest < ActiveSupport::TestCase
  test "normalize_url canonicalizes the cache key" do
    assert_equal "https://example.com/page",
      LinkEmbed.normalize_url("https://EXAMPLE.com/page#section")
    assert_equal "https://example.com/page",
      LinkEmbed.normalize_url("https://example.com/page/")
    assert_equal "https://example.com/",
      LinkEmbed.normalize_url("https://example.com")
    assert_equal "https://example.com:8443/page",
      LinkEmbed.normalize_url("https://example.com:8443/page")
    assert_equal "https://example.com/page?a=1&b=2",
      LinkEmbed.normalize_url("https://example.com/page?a=1&b=2")
    assert_equal "http://example.com/page",
      LinkEmbed.normalize_url("http://example.com:80/page")
  end

  test "normalize_url rejects garbage" do
    assert_nil LinkEmbed.normalize_url("not a url")
    assert_nil LinkEmbed.normalize_url("ftp://example.com/file")
    assert_nil LinkEmbed.normalize_url("")
    assert_nil LinkEmbed.normalize_url(nil)
  end

  test "for_reference resolves repeats to the same row" do
    first = LinkEmbed.for_reference("https://example.com/shared", url: "https://example.com/shared")
    second = LinkEmbed.for_reference("https://example.com/shared", url: "https://example.com/shared?utm=x")

    assert_equal first.id, second.id
    assert_equal 1, LinkEmbed.where(normalized_url: "https://example.com/shared").count
  end

  test "needs_fetch? follows the TTL" do
    embed = LinkEmbed.create!(normalized_url: "https://example.com/fresh")

    assert embed.needs_fetch?

    embed.update!(expires_at: 1.minute.from_now)
    assert_not embed.needs_fetch?

    travel_to 2.minutes.from_now do
      assert embed.reload.needs_fetch?
    end
  end

  test "claim_fetch_request! lets one caller win per window" do
    embed = LinkEmbed.create!(normalized_url: "https://example.com/claim")

    assert embed.claim_fetch_request!
    assert_not embed.claim_fetch_request!
    assert_not LinkEmbed.find(embed.id).claim_fetch_request!

    travel_to((LinkEmbed::FETCH_REQUEST_WINDOW + 1.second).from_now) do
      assert LinkEmbed.find(embed.id).claim_fetch_request!
    end
  end

  test "usable? needs a title or a description" do
    embed = LinkEmbed.create!(normalized_url: "https://example.com/usable")

    assert_not embed.usable?

    embed.update!(description: "excerpt")
    assert embed.usable?
  end

  test "linkedin? follows the post URL patterns" do
    assert LinkEmbed.new(normalized_url: "https://www.linkedin.com/posts/abc123").linkedin?
    assert LinkEmbed.new(normalized_url: "https://www.linkedin.com/feed/update/urn:li:share:1").linkedin?
    assert_not LinkEmbed.new(normalized_url: "https://example.com/page").linkedin?
  end
end
