# Receives Content Security Policy violation reports (report-uri, sent as
# application/csp-report, or the Reporting API's application/reports+json)
# and logs one line per violation. Unauthenticated, since browsers send
# reports without credentials guarantees, so it is rate-limited per IP,
# reads at most MAX_BODY bytes, and logs only the directive, blocked origin,
# and document path, never query strings. As an ActionController::API it
# loads no session, cookies, or forgery protection, so there is no
# authenticated state to forge requests against.
class ContentSecurityPolicyReportsController < ActionController::API
  MAX_BODY = 16.kilobytes
  MAX_VIOLATIONS = 10
  RATE_LIMIT = 20
  # Per process, so a report flood never touches the shared cache.
  RATE_LIMIT_STORE = ActiveSupport::Cache::MemoryStore.new

  rate_limit to: RATE_LIMIT, within: 1.minute, store: RATE_LIMIT_STORE, with: -> { head :too_many_requests }

  def create
    violations.first(MAX_VIOLATIONS).each do |violation|
      Rails.logger.warn "CSP violation: #{describe(violation)}"
    end

    head :no_content
  end

  private
    def violations
      body = request.body.read(MAX_BODY + 1).to_s
      return [] if body.bytesize > MAX_BODY

      Array.wrap(JSON.parse(body)).filter_map do |entry|
        next unless entry.is_a?(Hash)

        report = entry["csp-report"] || entry["body"]
        report if report.is_a?(Hash)
      end
    rescue JSON::ParserError
      []
    end

    def describe(report)
      directive = report["effective-directive"] || report["effectiveDirective"] || report["violated-directive"]
      blocked = report["blocked-uri"] || report["blockedURL"]
      document = report["document-uri"] || report["documentURL"]
      source = report["source-file"] || report["sourceFile"]
      line = report["line-number"] || report["lineNumber"]

      {
        directive: clean(directive, 60),
        blocked: origin_or_keyword(blocked),
        document: path_only(document),
        source: origin_or_keyword(source),
        line: line.to_s[/\A\d{1,7}\z/]
      }.compact.map { |key, value| "#{key}=#{value}" }.join(" ")
    end

    # Keywords such as "inline" or "eval" stay; URLs shrink to their origin.
    def origin_or_keyword(value)
      value = value.to_s
      return if value.blank?

      uri = URI.parse(value)
      uri.host ? "#{uri.scheme}://#{uri.host}#{":#{uri.port}" if uri.port && uri.port != uri.default_port}" : clean(value, 40)
    rescue URI::InvalidURIError
      clean(value, 40)
    end

    def path_only(value)
      URI.parse(value.to_s).path.presence&.then { |path| clean(path, 200) }
    rescue URI::InvalidURIError
      nil
    end

    def clean(value, limit)
      value.to_s.gsub(/[^\w\-.:\/@' ]/, "").truncate(limit).presence
    end
end
