require "test_helper"

class LinkEmbed::FetcherTest < ActiveSupport::TestCase
  test "stores OpenGraph metadata with a 24h TTL" do
    stub_page("https://example.com/page", <<~HTML)
      <html><head>
        <meta property="og:title" content="Example Title">
        <meta property="og:description" content="An example description.">
        <meta property="og:site_name" content="Example">
        <meta property="og:image" content="https://example.com/image.png">
      </head></html>
    HTML
    WebMock.stub_request(:head, "https://example.com/image.png")
      .to_return(status: 200, headers: { content_type: "image/png" })

    embed = LinkEmbed.create!(normalized_url: "https://example.com/page", url: "https://example.com/page")
    LinkEmbed::Fetcher.new(embed).fetch
    embed.reload

    assert_equal "Example Title", embed.title
    assert_equal "An example description.", embed.description
    assert_equal "Example", embed.site_name
    assert_equal "https://example.com/image.png", embed.image_url
    assert_nil embed.fetch_error
    assert_in_delta LinkEmbed::SUCCESS_TTL.from_now.to_i, embed.expires_at.to_i, 5
  end

  test "drops images without an image content type but keeps the text" do
    stub_page("https://example.com/noimg", <<~HTML)
      <html><head>
        <meta property="og:title" content="No Image">
        <meta property="og:description" content="The image is an HTML page.">
        <meta property="og:image" content="https://example.com/not-an-image">
      </head></html>
    HTML
    WebMock.stub_request(:head, "https://example.com/not-an-image")
      .to_return(status: 200, headers: { content_type: "text/html" })

    embed = LinkEmbed.create!(normalized_url: "https://example.com/noimg", url: "https://example.com/noimg")
    LinkEmbed::Fetcher.new(embed).fetch
    embed.reload

    assert_equal "No Image", embed.title
    assert_nil embed.image_url
    assert_nil embed.fetch_error
  end

  test "drops non-https and private images" do
    stub_page("https://example.com/plainimg", <<~HTML)
      <html><head>
        <meta property="og:title" content="Plain Image">
        <meta property="og:description" content="Served over http.">
        <meta property="og:image" content="http://example.com/image.png">
      </head></html>
    HTML

    embed = LinkEmbed.create!(normalized_url: "https://example.com/plainimg", url: "https://example.com/plainimg")
    LinkEmbed::Fetcher.new(embed).fetch

    assert_equal "Plain Image", embed.reload.title
    assert_nil embed.image_url
  end

  test "records a short negative TTL when the page has no usable metadata" do
    stub_page("https://example.com/empty", "<html><head></head><body>login wall</body></html>")

    embed = LinkEmbed.create!(normalized_url: "https://example.com/empty", url: "https://example.com/empty")
    LinkEmbed::Fetcher.new(embed).fetch
    embed.reload

    assert_equal "No preview available for this link", embed.fetch_error
    assert_in_delta LinkEmbed::NEGATIVE_TTL.from_now.to_i, embed.expires_at.to_i, 5
  end

  test "refuses private-network hosts without fetching" do
    stub_dns_resolution("10.0.0.5")

    embed = LinkEmbed.create!(normalized_url: "https://intranet.example/page", url: "https://intranet.example/page")
    LinkEmbed::Fetcher.new(embed).fetch
    embed.reload

    assert_equal "is not public", embed.fetch_error
    assert_not_nil embed.expires_at
    assert_not_requested :get, %r{intranet\.example}
  end

  test "refuses redirect targets on private networks" do
    WebMock.stub_request(:get, "https://example.com/redirect")
      .to_return(status: 302, headers: { location: "https://intranet.example/secret" })
    Resolv.stubs(:getaddresses).with("intranet.example").returns([ "10.0.0.5" ])

    embed = LinkEmbed.create!(normalized_url: "https://example.com/redirect", url: "https://example.com/redirect")
    LinkEmbed::Fetcher.new(embed).fetch

    assert_equal "Could not load this link", embed.reload.fetch_error
    assert_not_requested :get, "https://intranet.example/secret"
  end

  test "records network failures instead of raising" do
    WebMock.stub_request(:get, "https://example.com/down").to_raise(Errno::ECONNREFUSED)

    embed = LinkEmbed.create!(normalized_url: "https://example.com/down", url: "https://example.com/down")
    LinkEmbed::Fetcher.new(embed).fetch
    embed.reload

    assert_equal "Could not load this link", embed.fetch_error
    assert_in_delta LinkEmbed::NEGATIVE_TTL.from_now.to_i, embed.expires_at.to_i, 5
  end

  test "sends no cookies" do
    stub_page("https://example.com/page", "<html><head></head></html>")

    embed = LinkEmbed.create!(normalized_url: "https://example.com/page", url: "https://example.com/page")
    LinkEmbed::Fetcher.new(embed).fetch

    signatures = WebMock::RequestRegistry.instance.requested_signatures.hash.keys
    assert_not_empty signatures
    assert_empty signatures.select { |signature| signature.headers["Cookie"].present? }
  end

  private
    def stub_page(url, body)
      WebMock.stub_request(:get, url).to_return(status: 200, body: body, headers: { content_type: "text/html" })
    end
end
