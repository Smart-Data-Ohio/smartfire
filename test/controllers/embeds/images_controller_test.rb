require "test_helper"

class Embeds::ImagesControllerTest < ActionDispatch::IntegrationTest
  setup do
    @url = "https://images.example.com/photo.png"
  end

  test "show serves the proxied bytes with caching headers" do
    sign_in :david
    WebMock.stub_request(:get, @url)
      .to_return(status: 200, body: "PNG-BYTES", headers: { content_type: "image/png" })

    get Embeds::ImageProxy.signed_path(@url)

    assert_response :success
    assert_equal "image/png", response.content_type
    assert_equal "PNG-BYTES", response.body
    assert_equal "inline", response.headers["Content-Disposition"].to_s.split(";").first
    assert_includes response.headers["Cache-Control"], "max-age=3600"
    assert_includes response.headers["Cache-Control"], "public"
  end

  test "show requires sign-in" do
    get Embeds::ImageProxy.signed_path(@url)

    assert_response :redirect
  end

  test "show 404s tampered signatures with an empty body" do
    sign_in :david

    get embed_image_path(signed: "bogus")

    assert_response :not_found
    assert_empty response.body
    assert_not_requested :get, /.*/
  end

  test "show 404s SSRF targets without fetching" do
    sign_in :david
    stub_dns_resolution("127.0.0.1")

    get Embeds::ImageProxy.signed_path(@url)

    assert_response :not_found
    assert_empty response.body
    assert_not_requested :get, @url
  end

  test "show 502s upstream failures and wrong content types" do
    sign_in :david

    WebMock.stub_request(:get, @url).to_return(status: 500)
    get Embeds::ImageProxy.signed_path(@url)
    assert_response :bad_gateway
    assert_empty response.body

    WebMock.stub_request(:get, @url)
      .to_return(status: 200, body: "x", headers: { content_type: "text/html" })
    get Embeds::ImageProxy.signed_path(@url)
    assert_response :bad_gateway
    assert_empty response.body
  end

  test "message embeds point at the proxy instead of the remote host" do
    sign_in :david
    message = rooms(:watercooler).messages.create!(
      creator: users(:david),
      body: embed_body_for(href: "https://example.com/page", image: @url),
      client_message_id: "embed-proxy-check"
    )

    get room_url(message.room)

    assert_response :success
    assert_includes response.body, "/embeds/image/"
    assert_not_includes response.body, @url
  end

  private
    def embed_body_for(href:, image:)
      <<~HTML
        <div class="trix-content"><p>https://example.com/page</p>
        <action-text-attachment content-type="application/vnd.actiontext.opengraph-embed" href="#{href}" url="#{image}" filename="Example" caption="An example page"></action-text-attachment>
        </div>
      HTML
    end
end
