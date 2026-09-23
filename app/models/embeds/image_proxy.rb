require "net/http"
require "restricted_http/private_network_guard"

# Signed image proxy for link embed previews, so viewers' browsers never
# contact remote hosts: every embed <img> points here, and only here fetches
# the bytes. URLs are signed at render time (Embeds::ImageProxy.signed_path)
# and verified on arrival, so the endpoint is not an open proxy: it serves
# only image URLs the server itself rendered into an embed. Fetches resolve
# through RestrictedHTTP::PrivateNetworkGuard and pin each connection to
# the resolved public address, following Opengraph::Fetch: loopback and
# private destinations are refused, including redirect targets.
class Embeds::ImageProxy
  MAX_BODY_SIZE = 5.megabytes
  MAX_REDIRECTS = 10
  TIMEOUT = 5

  # Raster images only. SVG is deliberately excluded: an <img> cannot run
  # its scripts, but the same proxied URL navigated to directly would run
  # them as same-origin script.
  ALLOWED_CONTENT_TYPES = %w[
    image/jpeg image/png image/gif image/webp image/avif
    image/bmp image/x-icon image/vnd.microsoft.icon
  ].freeze

  # SystemCallError covers every Errno::* connection failure; IOError
  # covers EOFError from a dropped connection.
  TRANSPORT_ERRORS = [
    Net::OpenTimeout, Net::ReadTimeout, Net::WriteTimeout,
    Net::HTTPBadResponse, SocketError, SystemCallError, IOError,
    OpenSSL::SSL::SSLError
  ].freeze

  class FetchError < StandardError; end
  class Denied < FetchError; end
  class TooManyRedirects < FetchError; end
  class RedirectDenied < FetchError; end
  class UnusableResponse < FetchError; end

  Result = Data.define(:body, :content_type)

  class << self
    def signed_path(url)
      Rails.application.routes.url_helpers.embed_image_path(signed: verifier.generate(url.to_s))
    end

    def verified_url(signed)
      url = verifier.verified(signed.to_s)
      return unless url.is_a?(String)

      parsed = URI.parse(url)
      url if parsed.is_a?(URI::HTTP)
    rescue URI::InvalidURIError
      nil
    end

    def verifier
      Rails.application.message_verifier("embed_image")
    end
  end

  def fetch(url)
    parsed = URI.parse(url.to_s)
    raise Denied unless parsed.is_a?(URI::HTTP)

    request(parsed, ip: RestrictedHTTP::PrivateNetworkGuard.resolve(parsed.host))
  rescue URI::InvalidURIError
    raise Denied
  rescue *TRANSPORT_ERRORS => error
    raise UnusableResponse, error.class.name
  end

  private
    def request(url, ip:)
      MAX_REDIRECTS.times do
        Net::HTTP.start(url.host, url.port, ipaddr: ip, use_ssl: url.scheme == "https",
            open_timeout: TIMEOUT, read_timeout: TIMEOUT, write_timeout: TIMEOUT) do |http|
          http.request Net::HTTP::Get.new(url) do |response|
            if response.is_a?(Net::HTTPRedirection)
              url, ip = resolve_redirect(response["location"], url)
            else
              return body_if_acceptable(response)
            end
          end
        end
      end

      raise TooManyRedirects
    end

    def resolve_redirect(location, base)
      raise RedirectDenied if location.blank?

      url = base.merge(location.to_s)
      raise RedirectDenied unless url.is_a?(URI::HTTP)
      [ url, RestrictedHTTP::PrivateNetworkGuard.resolve(url.host) ]
    rescue URI::InvalidURIError
      raise RedirectDenied
    end

    def body_if_acceptable(response)
      raise UnusableResponse unless response.is_a?(Net::HTTPOK)
      raise UnusableResponse unless ALLOWED_CONTENT_TYPES.include?(response.content_type)
      raise UnusableResponse if response.content_length.to_i > MAX_BODY_SIZE

      body = size_restricted_body(response)
      raise UnusableResponse if body.nil?

      Result.new(body:, content_type: response.content_type)
    end

    def size_restricted_body(response)
      # The Content-Length header could be wrong or missing, so read in
      # chunks and bail past the cap.
      StringIO.new.tap do |body|
        response.read_body do |chunk|
          return nil if body.string.bytesize + chunk.bytesize > MAX_BODY_SIZE
          body << chunk
        end
      end.string
    end
end
