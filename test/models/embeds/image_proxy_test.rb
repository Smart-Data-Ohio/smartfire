require "test_helper"
require "restricted_http/private_network_guard"

class Embeds::ImageProxyTest < ActiveSupport::TestCase
  setup do
    @proxy = Embeds::ImageProxy.new
    @url = "https://images.example.com/photo.png"
  end

  test "signed urls round-trip and tampered ones verify to nil" do
    signed = Embeds::ImageProxy.signed_path(@url)

    assert signed.start_with?("/embeds/image/")
    assert_equal @url, Embeds::ImageProxy.verified_url(signed.split("/").last)
    assert_nil Embeds::ImageProxy.verified_url("#{signed.split('/').last}x")
    assert_nil Embeds::ImageProxy.verified_url("bogus")
    assert_nil Embeds::ImageProxy.verified_url("")
  end

  test "verified_url rejects urls the signer never signs" do
    [ "javascript:alert(1)", "data:image/png;base64,xx", "//images.example.com/x.png" ].each do |value|
      assert_nil Embeds::ImageProxy.verified_url(Embeds::ImageProxy.verifier.generate(value)),
        "expected #{value.inspect} to be rejected"
    end
  end

  test "fetch returns raster image bytes with their content type" do
    body = "PNG-BYTES"
    WebMock.stub_request(:get, @url)
      .to_return(status: 200, body: body, headers: { content_type: "image/png" })

    image = @proxy.fetch(@url)

    assert_equal body, image.body
    assert_equal "image/png", image.content_type
  end

  test "fetch refuses non-image and svg content types" do
    [ "text/html", "image/svg+xml" ].each do |content_type|
      WebMock.stub_request(:get, @url)
        .to_return(status: 200, body: "x", headers: { content_type: content_type })

      assert_raises Embeds::ImageProxy::UnusableResponse do
        @proxy.fetch(@url)
      end
    end
  end

  test "fetch refuses non-200 responses" do
    WebMock.stub_request(:get, @url).to_return(status: 404)

    assert_raises Embeds::ImageProxy::UnusableResponse do
      @proxy.fetch(@url)
    end
  end

  test "fetch refuses bodies past the size cap" do
    WebMock.stub_request(:get, @url)
      .to_return(status: 200, body: "x", headers: { content_type: "image/png", content_length: 6.megabytes })

    assert_raises Embeds::ImageProxy::UnusableResponse do
      @proxy.fetch(@url)
    end
  end

  test "fetch follows redirects to public hosts" do
    WebMock.stub_request(:get, @url)
      .to_return(status: 302, headers: { location: "https://cdn.example.com/photo.png" })
    WebMock.stub_request(:get, "https://cdn.example.com/photo.png")
      .to_return(status: 200, body: "PNG-BYTES", headers: { content_type: "image/png" })

    image = @proxy.fetch(@url)

    assert_equal "PNG-BYTES", image.body
  end

  test "fetch denies non-HTTP redirect targets" do
    WebMock.stub_request(:get, @url)
      .to_return(status: 302, headers: { location: "javascript:alert(1)" })

    assert_raises Embeds::ImageProxy::RedirectDenied do
      @proxy.fetch(@url)
    end
  end

  test "fetch gives up on redirect loops" do
    WebMock.stub_request(:get, @url)
      .to_return(status: 302, headers: { location: @url })

    assert_raises Embeds::ImageProxy::TooManyRedirects do
      @proxy.fetch(@url)
    end
  end

  test "fetch refuses private and unresolvable hosts" do
    stub_dns_resolution("127.0.0.1")
    assert_raises RestrictedHTTP::Violation do
      @proxy.fetch(@url)
    end

    stub_dns_resolution("10.1.2.3")
    assert_raises RestrictedHTTP::Violation do
      @proxy.fetch(@url)
    end

    stub_dns_failure
    assert_raises Surfguard::Unresolvable do
      @proxy.fetch(@url)
    end
  end

  test "fetch refuses hosts resolving to blocked IPv6 addresses" do
    [ "::1", "fd00::1", "fe80::1", "::ffff:127.0.0.1" ].each do |ip|
      stub_dns_resolution(ip)

      assert_raises RestrictedHTTP::Violation, "expected #{ip} to be refused" do
        @proxy.fetch(@url)
      end
    end

    assert_not_requested :get, @url
  end

  test "fetch refuses IP-literal urls pointing at blocked addresses" do
    [ "http://127.0.0.1/", "http://[::1]/", "http://2130706433/" ].each do |url|
      assert_raises RestrictedHTTP::Violation, "expected #{url} to be refused" do
        @proxy.fetch(url)
      end
    end

    assert_not_requested :get, /.*/
  end

  test "fetch does not follow redirects to private networks" do
    WebMock.stub_request(:get, @url)
      .to_return(status: 302, headers: { location: "https://internal.example.com/photo.png" })
    Resolv.stubs(:getaddresses).with("internal.example.com").returns([ "169.254.169.254" ])

    assert_raises RestrictedHTTP::Violation do
      @proxy.fetch(@url)
    end
  end

  test "fetch refuses non-HTTP urls without touching the network" do
    assert_raises Embeds::ImageProxy::Denied do
      @proxy.fetch("javascript:alert(1)")
    end

    assert_not_requested :get, /.*/
  end
end
