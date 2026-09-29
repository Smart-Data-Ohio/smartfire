# Verifies, with the reference app, the session cookies and CSRF tokens crates/kit issued
# (target/kit_security_rust_output.json, written by `cargo test -p campfire_kit --test rails_vectors`):
# a page rendered by the port whose form is submitted after switching back to Rails.
#
#   PARITY_OWNER=... parity/bin/reference runner reference-tools/kit/security_verify_rust.rb
#
# sessions#create answers an accepted post with 401 (the password is wrong) and a forged one
# with 422.
require_relative "../support"

class KitSecurityVerifyRust
  include ReferenceTools

  def initialize(path)
    @output = JSON.parse(File.read(path))
    @failures = []
    @checks = 0
    @ip = 0
  end

  def run
    reset_database!
    travel_to(Time.iso8601(@output.fetch("now")))

    fresh = @output.fetch("fresh")
    carried = @output.fetch("rails_session")
    check "the port's session cookie reads the same", read_cookie(:encrypted, "_campfire_session", fresh["session_cookie_raw"]), fresh["session"]

    [ [ "new visitor", fresh ], [ "Rails session", carried ] ].each do |label, issued|
      cookies = { "_campfire_session" => issued["session_cookie_raw"] }
      check "#{label}: form token", post(cookies, token: issued["form"]), 401
      check "#{label}: meta token as param", post(cookies, token: issued["meta"]), 401
      check "#{label}: meta token as header", post(cookies, header: issued["meta"]), 401
      check "#{label}: no token", post(cookies), 422
      check "#{label}: form token without the cookie", post({}, token: issued["form"]), 422
    end
    check "a token with another session's cookie", post({ "_campfire_session" => carried["session_cookie_raw"] }, token: fresh["form"]), 422

    travel_back
    report
  end

  private
    def post(cookies, token: nil, header: nil)
      @ip += 1
      headers = { "REMOTE_ADDR" => "10.9.0.#{@ip}" }
      headers["HTTP_X_CSRF_TOKEN"] = header if header
      params = { email_address: "david@example.com", password: "wrong" }
      params[:authenticity_token] = token if token
      status, _, _ = perform(:post, "/session", cookies: cookies, params: params, headers: headers)
      status
    end

    def check(label, actual, expected)
      @checks += 1
      @failures << "#{label}: expected #{expected.inspect}, got #{actual.inspect}" unless actual == expected
    end

    def report
      if @failures.empty?
        puts "kit security: #{@checks} checks passed"
      else
        puts @failures
        abort "kit security: #{@failures.size} of #{@checks} checks failed"
      end
    end
end

root = ENV.fetch("PARITY_WORK", File.expand_path("../..", __dir__))
KitSecurityVerifyRust.new(ARGV.first || File.join(root, "target/kit_security_rust_output.json")).run
