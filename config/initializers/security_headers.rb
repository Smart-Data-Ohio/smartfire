# Application-wide security headers, pinned here rather than inherited
# from the framework defaults so a Rails upgrade cannot silently weaken
# them. HSTS is production-only via force_ssl; see
# config/environments/production.rb and docs/security.md.
Rails.application.configure do
  config.action_dispatch.default_headers.merge!(
    "X-Content-Type-Options" => "nosniff",
    "Referrer-Policy" => "strict-origin-when-cross-origin",
    # Huddles need the camera, microphone, and screen sharing. Set as a
    # raw header rather than through config.permissions_policy because
    # the required value also names notifications, which is not a
    # Permissions-Policy directive (the Notifications API is gated by
    # its own user prompt, not by policy) and the Rails DSL rejects
    # unknown directives. Browsers ignore the unknown entry.
    "Permissions-Policy" => "camera=(self), display-capture=(self), microphone=(self), notifications=(self)"
  )
end
