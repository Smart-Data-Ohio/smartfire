# Verifies, with our Rails app, the values crates/rails_compat generated for Smartfire's own
# contracts (target/rails_compat_smartfire_rust_output.json, written by `cargo test -p
# rails_compat`): Active Record encrypted columns (read back through the models, including from
# real rows), the Calendar cleanup credentials, the named verifiers, the device and two-factor
# cookies, signed ids, LiveKit tokens and webhook signatures. Each goes through the app's own code
# path, so data Rust writes stays readable after a rollback to Rails.
#
#   reference-tools/run.sh reference-tools/rails_compat_smartfire_verify_rust.rb [path/to/output.json]
require_relative "support"

class RailsCompatSmartfireVerifyRust
  include ReferenceTools

  module CaptureWebhookPost
    def start(host, *args, **kwargs, &block)
      return super unless host == "hooks.example.com"

      http = Object.new
      http.define_singleton_method(:request) do |request|
        Thread.current[:rails_compat_webhook_request] = request
        Net::HTTPOK.new("1.1", "200", "OK")
      end
      block.call(http)
    end
  end

  module ResolveAnything
    def resolve(host) = "93.184.216.34"
  end

  def initialize(path)
    @output = JSON.parse(File.read(path))
    @failures = []
    @checks = 0
  end

  def run
    ENV["LIVEKIT_API_KEY"] = @output.dig("livekit", "api_key")
    ENV["LIVEKIT_API_SECRET"] = @output.dig("livekit", "api_secret")
    ActiveJob::Base.queue_adapter = :test
    ActionCable.server.instance_variable_set(:@pubsub, ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))

    reset_database!
    @now = Time.iso8601(@output.fetch("now"))
    travel_to(@now)

    verify_ar_encryption
    verify_ar_encryption_in_rows
    verify_calendar_credentials
    verify_oauth_states
    verify_embed_images
    verify_bot_reply
    verify_cookies
    verify_signed_ids
    verify_livekit_tokens
    verify_webhooks

    travel_back
    report
  end

  private
    def check(label, actual, expected)
      @checks += 1
      @failures << "#{label}: expected #{expected.inspect}, got #{actual.inspect}" unless actual == expected
    end

    def outcome
      yield
    rescue Exception => error # rubocop:disable Lint/RescueException
      "raised #{error.class}"
    end

    # --- Active Record encryption -----------------------------------------------------------------

    def verify_ar_encryption
      @output["ar_encryption"].each do |entry|
        type = entry["model"].constantize.type_for_attribute(entry["attribute"])
        value = outcome { type.deserialize(entry["ciphertext"]) }
        label = "AR #{entry["case"]}"
        check "#{label} bytes", value.is_a?(String) ? value.unpack1("H*") : value, entry["value_hex"]
        check "#{label} encoding", value.is_a?(String) ? value.encoding.name : value, entry["value_encoding"]
        compressed = JSON.parse(entry["ciphertext"]).dig("h", "c") == true
        check "#{label} compressed as Rails would", compressed, entry["value_hex"].size / 2 > 140
      end
    end

    # Rust's ciphertext written straight into real rows, then read through the models.
    def verify_ar_encryption_in_rows
      credential = TwoFactorCredential.create!(user: @david, secret: TwoFactorCredential.generate_secret)
      google = GoogleAccount.create!(user: @david, email: "david@example.com", refresh_token: "r", access_token: "a",
        access_token_expires_at: @now + 1.hour)
      rows = { "TwoFactorCredential" => credential, "GoogleAccount" => google }

      @output["ar_encryption"].select { |entry| rows.key?(entry["model"]) }.each do |entry|
        record = rows.fetch(entry["model"])
        connection = record.class.connection
        connection.execute("UPDATE #{record.class.table_name} SET #{entry["attribute"]} = #{connection.quote(entry["ciphertext"])} WHERE id = #{record.id}")
        value = outcome { record.class.find(record.id).public_send(entry["attribute"]) }
        check "AR row #{entry["case"]}", value.is_a?(String) ? value.unpack1("H*") : value, entry["value_hex"]
      end
    end

    # --- Calendar::DisconnectCleanupJob credentials -------------------------------------------------

    def verify_calendar_credentials
      blob = @output.dig("calendar", "blob")
      check "calendar credentials", Calendar::DisconnectCleanupJob.decrypt_credentials(blob), @output.dig("calendar", "expected")
      check "calendar credentials just before a day",
        at(@now + 1.day - 1.second) { Calendar::DisconnectCleanupJob.decrypt_credentials(blob) }, @output.dig("calendar", "expected")
      check "calendar credentials after a day", at(@now + 1.day) { Calendar::DisconnectCleanupJob.decrypt_credentials(blob) }, nil
      travel_to(@now)
    end

    # --- Named verifiers -------------------------------------------------------------------------------

    def state_verifiers
      {
        "google_sign_in_state" => Sessions::GoogleController.new.send(:google_sign_in_state_verifier),
        "google_oauth_state" => Google::ConnectionsController.new.send(:state_verifier),
        "github_app_oauth_state" => Github::AppConnectionsController.new.send(:state_verifier),
        "slack_oauth_state" => Slack::OAuthController.new.send(:state_verifier)
      }
    end

    def verify_oauth_states
      verifiers = state_verifiers
      check "every OAuth state verifier covered", @output["oauth_states"].map { |state| state["name"] }.sort, verifiers.keys.sort
      @output["oauth_states"].each do |state|
        verifiers.each do |name, verifier|
          expected = name == state["name"] ? state["raw_state"] : nil
          check "#{state["name"]} state under #{name}", verifier.verified(state["message"]), expected
        end
      end
    end

    def verify_embed_images
      @output["embed_image"].each do |embed|
        check "embed_image #{embed["url"]}", Embeds::ImageProxy.verifier.verified(embed["signed"]), embed["url"]
        expected_url = embed["url"].start_with?("http") && embed["url"].ascii_only? ? embed["url"] : nil
        check "embed_image verified_url #{embed["url"]}", Embeds::ImageProxy.verified_url(embed["signed"]), expected_url
      end
    end

    def verify_bot_reply
      reply = @output["bot_reply"]
      bot = User.create_bot!(name: "Rust Bot")
      check "bot id matches the one Rust signed", bot.id, reply["bot_id"]
      check "room id matches the one Rust signed", @room.id, reply["room_id"]
      @room.memberships.find_or_create_by!(user: bot)
      check "bot reply token", User.authenticate_bot_reply_token(reply["token"], room_id: @room.id), bot
      check "bot reply token for another room", User.authenticate_bot_reply_token(reply["token"], room_id: @room.id + 1), nil
      check "bot reply token at expiry", at(@now + 15.minutes) { User.authenticate_bot_reply_token(reply["token"], room_id: @room.id) }, nil
      travel_to(@now)
    end

    # --- Cookies ------------------------------------------------------------------------------------

    def verify_cookies
      @output["cookies"].each do |cookie|
        name, raw = cookie["name"], cookie["raw"]
        check "#{name} cookie", read_cookie(:signed, name, raw), cookie["value"]
        wire = Rack::Utils.unescape(cookie["set_cookie"][/\A#{Regexp.escape(name)}=([^;]*)/, 1].to_s)
        check "#{name} Set-Cookie value", read_cookie(:signed, name, wire), cookie["value"]
        other = name == "device_id" ? "two_factor_remember" : "device_id"
        check "#{name} cookie under #{other}", read_cookie(:signed, other, raw), nil
      end
      remember = @output["cookies"].find { |cookie| cookie["name"] == "two_factor_remember" }
      check "two_factor_remember after 30 days", at(@now + 30.days) { read_cookie(:signed, "two_factor_remember", remember["raw"]) }, nil
      travel_to(@now)
    end

    # --- Signed ids -----------------------------------------------------------------------------------

    def verify_signed_ids
      ids = @output["signed_ids"]
      check "signed ids are for David", ids["user_id"], @david.id
      check "transfer id", User.find_by_transfer_id(ids["transfer_id"]), @david
      check "transfer id at 4 hours", at(@now + 4.hours) { User.find_by_transfer_id(ids["transfer_id"]) }, nil
      travel_to(@now)
      check "avatar token", outcome { User.from_avatar_token(ids["avatar_token"]) }, @david
      check "avatar token as transfer id", User.find_by_transfer_id(ids["avatar_token"]), nil
      check "transfer id as avatar token", outcome { User.from_avatar_token(ids["transfer_id"]) }, "raised ActiveSupport::MessageVerifier::InvalidSignature"
    end

    # --- LiveKit ----------------------------------------------------------------------------------------

    def verify_livekit_tokens
      @output.dig("livekit", "tokens").each do |token|
        check "LiveKit token #{token["identity"]}", outcome { Huddle::TokenVerifier.new(token["token"]).coordinates },
          { identity: token["identity"], room_name: token["room_name"] }
        check "LiveKit token #{token["identity"]} after it expires",
          at(@now + Huddle::TOKEN_TTL) { outcome { Huddle::TokenVerifier.new(token["token"]).coordinates } }, "raised Huddle::TokenVerifier::Invalid"
        travel_to(@now)
      end
    end

    # --- Webhooks ---------------------------------------------------------------------------------------

    def verify_webhooks
      Net::HTTP.singleton_class.prepend(CaptureWebhookPost)
      RestrictedHTTP::PrivateNetworkGuard.singleton_class.prepend(ResolveAnything)
      webhook = Webhook.new(url: "https://hooks.example.com/deliver")
      @output["webhooks"].each do |entry|
        webhook.post_payload(entry["body"], secret: entry["secret"])
        request = Thread.current[:rails_compat_webhook_request]
        rails_headers = entry["headers"].map { |name, _| [ name, request[name] ] }
        check "webhook headers for #{entry["body"].inspect}", entry["headers"], rails_headers
      end
    end

    def report
      puts "#{@checks} checks, #{@failures.size} failures"
      @failures.each { |failure| puts "  FAIL #{failure}" }
      # Exiting inside `rails runner`'s executor trips the error reporter, so exit afterwards.
      status = @failures.empty?
      at_exit { exit(status) }
    end
end

default = File.expand_path("../target/rails_compat_smartfire_rust_output.json", __dir__)
default = "/work/target/rails_compat_smartfire_rust_output.json" unless File.exist?(default)
RailsCompatSmartfireVerifyRust.new(ARGV.first || default).run
