require "net/http"
require "timeout"
require "restricted_http/private_network_guard"

class Opengraph::Fetch
  ALLOWED_DOCUMENT_CONTENT_TYPE = "text/html"
  MAX_BODY_SIZE = 5.megabytes
  MAX_REDIRECTS = 10
  TIMEOUT = 5

  class TooManyRedirectsError < StandardError; end
  class RedirectDeniedError < StandardError; end

  # max_redirects caps how many redirects are followed before
  # TooManyRedirectsError (the default keeps the legacy unfurl budget).
  # deadline, in seconds, bounds the whole fetch — every redirect and
  # every read — with Timeout::Error. Callers that record failures
  # instead of raising (LinkEmbed::Fetcher) rescue it like any network
  # error. Nil means no overall deadline: each operation still has its
  # own open/read/write timeout.
  def fetch_document(url, ip: RestrictedHTTP::PrivateNetworkGuard.resolve(url.host), max_redirects: MAX_REDIRECTS, deadline: nil)
    with_deadline(deadline) do
      request(url, Net::HTTP::Get, ip: ip, max_redirects: max_redirects, deadline: deadline) do |response|
        return body_if_acceptable(response)
      end
    end
  end

  def fetch_content_type(url, ip: RestrictedHTTP::PrivateNetworkGuard.resolve(url.host), max_redirects: MAX_REDIRECTS, deadline: nil)
    with_deadline(deadline) do
      request(url, Net::HTTP::Head, ip: ip, max_redirects: max_redirects, deadline: deadline) do |response|
        return response["Content-Type"]
      end
    end
  end

  private
    def with_deadline(deadline)
      return yield if deadline.nil?

      Timeout.timeout(deadline, Timeout::Error, "fetch deadline exceeded") { yield }
    end

    def request(url, request_class, ip:, max_redirects:, deadline:)
      (max_redirects + 1).times do
        Net::HTTP.start(url.host, url.port, ipaddr: ip, use_ssl: url.scheme == "https",
            open_timeout: TIMEOUT, read_timeout: TIMEOUT, write_timeout: TIMEOUT) do |http|
          # Net::HTTP retries idempotent requests once on Timeout::Error.
          # With an overall deadline armed that retry would swallow the
          # deadline's own fire — Timeout is one-shot, so the retried
          # attempt would run unbounded — hence no silent retries here.
          http.max_retries = 0 unless deadline.nil?
          http.request request_class.new(url) do |response|
            if response.is_a?(Net::HTTPRedirection)
              url, ip = resolve_redirect(response["location"], url)
            else
              yield response
            end
          end
        end
      end

      raise TooManyRedirectsError
    end

    def resolve_redirect(location, base)
      raise RedirectDeniedError if location.blank?

      url = base.merge(location.to_s)
      raise RedirectDeniedError unless url.is_a?(URI::HTTP)
      [ url, RestrictedHTTP::PrivateNetworkGuard.resolve(url.host) ]
    rescue URI::InvalidURIError
      raise RedirectDeniedError
    end

    def body_if_acceptable(response)
      size_restricted_body(response) if response_valid?(response)
    end

    def size_restricted_body(response)
      # We've already checked the Content-Length header, to try to avoid reading
      # the body of any large responses. But that header could be wrong or
      # missing. To be on the safe side, we'll read the body in chunks, and bail
      # if it runs over our size limit.
      StringIO.new.tap do |body|
        response.read_body do |chunk|
          return nil if body.string.bytesize + chunk.bytesize > MAX_BODY_SIZE
          body << chunk
        end
      end.string
    end

    def response_valid?(response)
      status_valid?(response) && content_type_valid?(response) && content_length_valid?(response)
    end

    def status_valid?(response)
      response.is_a?(Net::HTTPOK)
    end

    def content_type_valid?(response)
      response.content_type == ALLOWED_DOCUMENT_CONTENT_TYPE
    end

    def content_length_valid?(response)
      response.content_length.to_i <= MAX_BODY_SIZE
    end
end
