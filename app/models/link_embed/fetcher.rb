class LinkEmbed::Fetcher
  ALLOWED_IMAGE_CONTENT_TYPES = %w[ image/jpeg image/png image/gif image/webp image/avif ].freeze

  # Fetches one embed's OpenGraph/Twitter-card metadata and persists the
  # card fields on the record.
  #
  # The page fetch goes through Opengraph::Location/Opengraph::Fetch, so it
  # inherits their SSRF guard (PrivateNetworkGuard hostname resolution with
  # the resolved address pinned for the connection, redirects re-resolved),
  # explicit 5 s open/read/write timeouts, the 5 MB body cap, and no cookies
  # (no Cookie header is ever set). Expected failures — private hosts,
  # login-gated pages with no usable tags, network problems — are recorded
  # as fetch_error with a short negative TTL instead of raising.
  def initialize(embed)
    @embed = embed
  end

  def fetch
    location = Opengraph::Location.new(@embed.display_url)

    unless location.valid?
      return record_negative(location.errors[:url].first || "Could not load this link")
    end

    html = location.read_html
    return record_negative("Could not load this link") if html.blank?

    metadata = LinkEmbed::MetadataParser.parse(html, base_url: @embed.display_url)
    image_url = valid_image_url(metadata.image_url)

    if metadata.title.blank? && metadata.description.blank?
      record_negative("No preview available for this link")
    else
      @embed.update!(
        site_name: metadata.site_name,
        title: metadata.title,
        description: metadata.description,
        image_url: image_url,
        fetched_at: Time.current,
        fetch_error: nil,
        expires_at: LinkEmbed::SUCCESS_TTL.from_now
      )
    end
  rescue StandardError => error
    Rails.logger.warn "LinkEmbed::Fetcher failed for #{@embed.normalized_url}: #{error.class}"
    record_negative("Could not load this link")
  end

  private
    def record_negative(message)
      @embed.update!(
        fetched_at: Time.current,
        fetch_error: message,
        expires_at: LinkEmbed::NEGATIVE_TTL.from_now
      )
    end

    # Card images render directly in the browser (img-src is https-wide),
    # so only public https targets with an image content type survive.
    # Anything else is dropped and the card renders without its image.
    def valid_image_url(value)
      return if value.blank?

      parsed = URI.parse(value.to_s)
      return unless parsed.is_a?(URI::HTTPS) && parsed.host.present?

      location = Opengraph::Location.new(value.to_s)
      return unless location.valid?

      content_type = location.fetch_content_type&.downcase
      value.to_s if content_type.in?(ALLOWED_IMAGE_CONTENT_TYPES)
    rescue URI::InvalidURIError
      nil
    end
end
