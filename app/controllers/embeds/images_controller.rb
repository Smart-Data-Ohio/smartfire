# Serves signed link-embed images fetched through Embeds::ImageProxy.
# Signed-in members only: possession of a signed URL alone grants nothing.
# Denials (bad signature, non-HTTP URL, SSRF refusal) answer 404 with an
# empty body; upstream fetch failures answer 502, also empty. Bodies are
# cacheable for an hour: the signature binds the source URL, and browsers
# re-request with a fresh page render anyway.
class Embeds::ImagesController < ApplicationController
  def show
    url = Embeds::ImageProxy.verified_url(params[:signed])
    return head :not_found if url.nil?

    image = Embeds::ImageProxy.new.fetch(url)

    expires_in 1.hour, public: true
    send_data image.body, type: image.content_type, disposition: "inline"
  rescue Embeds::ImageProxy::Denied, RestrictedHTTP::Violation, Surfguard::Unresolvable
    head :not_found
  rescue Embeds::ImageProxy::FetchError
    head :bad_gateway
  end
end
