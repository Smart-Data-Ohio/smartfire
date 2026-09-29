# Generates vectors/rails_compat_smartfire.json: golden vectors for the signing, encryption and
# token contracts Smartfire added on top of stock Campfire (crates/rails_compat: ar_encryption,
# message encryptors, named verifiers, cookies, signed ids, JWT, webhook signatures). Every expected
# value comes from our app's own code paths (models, controllers, services), never from a
# reimplementation. Where the app draws randomness (IVs, JWT ids), the value it drew is recorded.
#
#   reference-tools/run.sh reference-tools/rails_compat_smartfire_vectors.rb
require_relative "support"
require "zlib"

class RailsCompatSmartfireVectors
  include ReferenceTools

  LIVEKIT_API_KEY = "APIparityKey"
  LIVEKIT_API_SECRET = "parity-livekit-secret-0123456789abcdef0123456789"
  GOOGLE_CLIENT_ID = "1234567890-parity.apps.googleusercontent.com"
  GOOGLE_SIGN_IN_DOMAINS = " Example.com, smartdata.test ,not a domain,,example.com, -bad.test, sub.example.org "
  GITHUB_WEBHOOK_SECRET = "github-webhook-secret-parity"

  def generate
    ENV["LIVEKIT_API_KEY"] = LIVEKIT_API_KEY
    ENV["LIVEKIT_API_SECRET"] = LIVEKIT_API_SECRET
    ENV["GOOGLE_CLIENT_ID"] = GOOGLE_CLIENT_ID
    ENV["GOOGLE_SIGN_IN_DOMAINS"] = GOOGLE_SIGN_IN_DOMAINS
    ActiveJob::Base.queue_adapter = :test

    reset_database!
    Rails.application.eager_load!

    travel_to(NOW)
    begin
      {
        "secret_key_base" => app.secret_key_base,
        "now" => iso(NOW),
        "key_generator" => key_generator_vectors,
        "ar_encryption" => ar_encryption_vectors,
        "message_encryptors" => message_encryptor_vectors,
        "verifiers" => verifier_vectors,
        "cookies" => cookie_vectors,
        "signed_ids" => signed_id_vectors,
        "jwt" => { "livekit" => livekit_vectors, "google" => google_vectors },
        "webhooks" => webhook_vectors
      }
    ensure
      travel_back
    end
  end

  private
    def tamper_last(string)
      string[0..-2] + (string[-1] == "0" ? "1" : "0")
    end

    def tamper_first(string)
      (string[0] == "A" ? "B" : "A") + string[1..]
    end

    def outcome
      { "value" => yield }
    rescue Exception => error # rubocop:disable Lint/RescueException
      { "error" => error.class.name }
    end

    # --- The app key generator ------------------------------------------------------------------

    def key_generator_vectors
      generator = app.key_generator.instance_variable_get(:@key_generator)
      {
        "hash_digest_class" => generator.instance_variable_get(:@hash_digest_class).name,
        "iterations" => generator.instance_variable_get(:@iterations),
        "configured_hash_digest_class" => app.config.active_support.key_generator_hash_digest_class&.name,
        "keys" => [ [ "active_record_encryption/primary", 32 ], [ "calendar/disconnect-cleanup", 32 ], [ "bot_reply", 64 ] ].map do |salt, length|
          { "salt" => salt, "length" => length, "key_hex" => app.key_generator.generate_key(salt, length).unpack1("H*") }
        end
      }
    end

    # --- Active Record encryption ---------------------------------------------------------------

    module FixedIv
      def random_iv
        if (iv = Thread.current[:rails_compat_fixed_iv])
          self.iv = iv
          iv
        else
          super
        end
      end
    end
    OpenSSL::Cipher.prepend(FixedIv)

    def with_fixed_iv(iv)
      Thread.current[:rails_compat_fixed_iv] = iv
      yield
    ensure
      Thread.current[:rails_compat_fixed_iv] = nil
    end

    ENCRYPTED_ATTRIBUTES = [
      [ "GoogleAccount", "refresh_token" ], [ "GoogleAccount", "access_token" ],
      [ "GithubConnectedAccount", "access_token" ], [ "GithubConnectedAccount", "refresh_token" ],
      [ "FizzyConnectedAccount", "access_token" ],
      [ "SlackWorkspace", "client_secret" ], [ "SlackConnection", "access_token" ],
      [ "Agent", "webhook_signing_secret" ], [ "Webhook", "signing_secret" ],
      [ "TwoFactorCredential", "secret" ], [ "TwoFactorSetupSecret", "secret" ]
    ]

    # Realistic lengths and character sets per column (provider tokens, our own hex secrets, ROTP
    # secrets), but never a real provider's token prefix (xox*, gh*_, ya29., 1//0, ...): secret
    # scanners reject pushes that contain them. The Google access token stays over 140 bytes so
    # it's compressed.
    def realistic_values
      totp = TwoFactorCredential.generate_secret
      {
        [ "GoogleAccount", "refresh_token" ] => "FAKE-google-refresh-CgYIARAAGBASNwF-L9Ir".ljust(103, "x"),
        [ "GoogleAccount", "access_token" ] => "FAKE-google-access-AfB_byParityAccessToken".ljust(220, "Q"),
        [ "GithubConnectedAccount", "access_token" ] => ("FAKE-github-access-" + "A1b2C3d4E5" * 2).ljust(40, "a"),
        [ "GithubConnectedAccount", "refresh_token" ] => ("FAKE-github-refresh-" + "Z9y8X7w6V5" * 6).ljust(80, "z"),
        [ "FizzyConnectedAccount", "access_token" ] => "FAKE-fizzy-access-".ljust(49, "k"),
        [ "SlackWorkspace", "client_secret" ] => "0123456789abcdef0123456789abcdef",
        [ "SlackConnection", "access_token" ] => "FAKE-slack-user-1234567890-1234567890123-".ljust(76, "f"),
        [ "Agent", "webhook_signing_secret" ] => Agent.generate_webhook_signing_secret,
        [ "Webhook", "signing_secret" ] => SecureRandom.hex(32),
        [ "TwoFactorCredential", "secret" ] => totp,
        [ "TwoFactorSetupSecret", "secret" ] => TwoFactorCredential.generate_secret
      }
    end

    EDGE_VALUES = [
      "", "a", "x" * 140, "x" * 141, "é" * 70, "é" * 71, "☃ unicode <&> \"quoted\"\n" * 10,
      "long refresh token " * 60, "\u0000binary-ish\u0001"
    ]

    def value_for_database(model_name, attribute, value)
      record = model_name.constantize.new(attribute => value)
      record.instance_variable_get(:@attributes)[attribute].value_for_database
    end

    def encryption_case(model_name, attribute, value, label)
      ciphertext = value_for_database(model_name, attribute, value)
      iv = Digest::SHA256.hexdigest(label)[0, 24]
      fixed = with_fixed_iv([ iv ].pack("H*")) { value_for_database(model_name, attribute, value) }
      {
        "case" => label, "model" => model_name, "attribute" => attribute,
        "value" => (utf8 = value.dup.force_encoding("UTF-8")).valid_encoding? ? utf8 : nil,
        "value_hex" => value.unpack1("H*"), "value_encoding" => value.encoding.name,
        "ciphertext" => ciphertext,
        "fixed_iv_hex" => iv, "fixed_iv_ciphertext" => fixed,
        "compressed" => JSON.parse(ciphertext).dig("h", "c") == true,
        "decrypted_hex" => model_name.constantize.type_for_attribute(attribute).deserialize(ciphertext).unpack1("H*")
      }
    end

    def ar_encryption_vectors
      config = ActiveRecord::Encryption.config
      key = ActiveRecord::Encryption.key_provider.encryption_key

      encrypted_attributes = ActiveRecord::Base.descendants.reject(&:abstract_class?).flat_map do |klass|
        klass.encrypted_attributes.to_a.map { |attribute| [ klass.name, attribute.to_s ] }
      end.uniq.sort

      schemes = encrypted_attributes.map do |model_name, attribute|
        scheme = model_name.constantize.type_for_attribute(attribute).scheme
        { "model" => model_name, "attribute" => attribute, "table" => model_name.constantize.table_name,
          "deterministic" => scheme.deterministic?, "previous_schemes" => scheme.previous_schemes.size,
          "support_unencrypted_data" => scheme.support_unencrypted_data? }
      end

      values = realistic_values
      encryptions = ENCRYPTED_ATTRIBUTES.map do |model_name, attribute|
        encryption_case(model_name, attribute, values.fetch([ model_name, attribute ]), "#{model_name}##{attribute} realistic")
      end
      EDGE_VALUES.each_with_index do |value, index|
        encryptions << encryption_case("GoogleAccount", "access_token", value, "edge #{index}: #{value[0, 20].inspect} (#{value.bytesize} bytes)")
      end
      binary = "\xFF\xFEnot utf-8".b
      encryptions << encryption_case("Webhook", "signing_secret", binary, "binary, not UTF-8")
      encryptions << encryption_case("Webhook", "signing_secret", "ascii".encode("US-ASCII"), "US-ASCII encoded")

      {
        "config" => {
          "primary_key_hex" => config.primary_key.unpack1("H*"),
          "deterministic_key_hex" => config.deterministic_key.unpack1("H*"),
          "key_derivation_salt_hex" => config.key_derivation_salt.unpack1("H*"),
          "hash_digest_class" => config.hash_digest_class.name,
          "key_generator_hash_digest_class" => ActiveRecord::Encryption.key_generator.hash_digest_class.name,
          "derived_key_hex" => key.secret.unpack1("H*"),
          "key_id" => key.id,
          "store_key_references" => config.store_key_references,
          "support_unencrypted_data" => config.support_unencrypted_data,
          "compressor" => config.compressor.name,
          "previous_schemes" => config.previous_schemes.size
        },
        "encrypted_attributes" => schemes,
        "encryptions" => encryptions,
        "records" => record_round_trips,
        "decrypt" => ar_decrypt_vectors(encryptions)
      }
    end

    # Real saves: what lands in the column, and what the model reads back.
    def record_round_trips
      credential = TwoFactorCredential.create!(user: @david, secret: TwoFactorCredential.generate_secret)
      google = GoogleAccount.create!(user: @david, email: "david@example.com",
        refresh_token: "FAKE-record-refresh".ljust(108, "r"), access_token: "FAKE-record-access".ljust(217, "a"),
        access_token_expires_at: NOW + 1.hour)
      [ [ credential, "secret" ], [ google, "refresh_token" ], [ google, "access_token" ] ].map do |record, attribute|
        raw = record.class.connection.select_value("SELECT #{attribute} FROM #{record.class.table_name} WHERE id = #{record.id}")
        reloaded = record.class.find(record.id).public_send(attribute)
        { "model" => record.class.name, "attribute" => attribute, "column_value" => raw,
          "read_value" => reloaded.dup.force_encoding("UTF-8"), "read_encoding" => reloaded.encoding.name }
      end
    end

    def message_json(ciphertext)
      JSON.parse(ciphertext)
    end

    def rewrite(ciphertext)
      data = message_json(ciphertext)
      yield data
      JSON.dump(data)
    end

    def b64(value)
      Base64.strict_encode64(value)
    end

    def ar_decrypt_vectors(encryptions)
      short = encryptions.find { |e| e["case"] == "GoogleAccount#refresh_token realistic" }["ciphertext"]
      long = encryptions.find { |e| e["compressed"] }["ciphertext"]
      other_primary = "\x01".b * 32
      other_provider = ActiveRecord::Encryption::DerivedSecretKeyProvider.new(other_primary)
      key_id = ActiveRecord::Encryption.key_provider.encryption_key.id
      encryptor = ActiveRecord::Encryption.encryptor

      cases = encryptions.map { |e| [ "valid #{e["case"]}", e["ciphertext"] ] } +
        encryptions.map { |e| [ "valid fixed iv #{e["case"]}", e["fixed_iv_ciphertext"] ] } + [
        [ "encrypted under another primary key", encryptor.encrypt("secret-token", key_provider: other_provider) ],
        [ "tampered payload", rewrite(short) { |d| d["p"] = b64(Base64.strict_decode64(d["p"]).tap { |s| s.setbyte(0, s.getbyte(0) ^ 1) }) } ],
        [ "tampered compressed payload", rewrite(long) { |d| d["p"] = b64(Base64.strict_decode64(d["p"]).tap { |s| s.setbyte(3, s.getbyte(3) ^ 1) }) } ],
        [ "tampered iv", rewrite(short) { |d| d["h"]["iv"] = b64(Base64.strict_decode64(d["h"]["iv"]).tap { |s| s.setbyte(0, s.getbyte(0) ^ 1) }) } ],
        [ "tampered auth tag", rewrite(short) { |d| d["h"]["at"] = b64(Base64.strict_decode64(d["h"]["at"]).tap { |s| s.setbyte(15, s.getbyte(15) ^ 1) }) } ],
        [ "truncated auth tag", rewrite(short) { |d| d["h"]["at"] = b64(Base64.strict_decode64(d["h"]["at"])[0, 15]) } ],
        [ "missing auth tag", rewrite(short) { |d| d["h"].delete("at") } ],
        [ "missing iv", rewrite(short) { |d| d["h"].delete("iv") } ],
        [ "short iv", rewrite(short) { |d| d["h"]["iv"] = b64(Base64.strict_decode64(d["h"]["iv"])[0, 11]) } ],
        [ "long iv", rewrite(short) { |d| d["h"]["iv"] = b64(Base64.strict_decode64(d["h"]["iv"]) + "x") } ],
        [ "missing headers", rewrite(short) { |d| d.delete("h") } ],
        [ "missing payload", rewrite(short) { |d| d.delete("p") } ],
        [ "numeric payload", rewrite(short) { |d| d["p"] = 1 } ],
        [ "payload not base64", rewrite(short) { |d| d["p"] = "!!!" } ],
        [ "payload url-safe base64", rewrite(short) { |d| d["p"] = d["p"].tr("+/", "-_") } ],
        [ "compressed flag on uncompressed data", rewrite(short) { |d| d["h"]["c"] = true } ],
        [ "compressed flag false on compressed data", rewrite(long) { |d| d["h"]["c"] = false } ],
        [ "compressed flag removed", rewrite(long) { |d| d["h"].delete("c") } ],
        [ "matching key id header", rewrite(short) { |d| d["h"]["i"] = b64(key_id) } ],
        [ "other key id header", rewrite(short) { |d| d["h"]["i"] = b64("ffff") } ],
        [ "encoding header US-ASCII", rewrite(short) { |d| d["h"]["e"] = b64("US-ASCII") } ],
        [ "unknown header", rewrite(short) { |d| d["h"]["zz"] = b64("anything") } ],
        [ "unknown numeric header", rewrite(short) { |d| d["h"]["zz"] = 5 } ],
        [ "header not base64", rewrite(short) { |d| d["h"]["zz"] = "!!!" } ],
        [ "header array", rewrite(short) { |d| d["h"]["zz"] = [ 1 ] } ],
        [ "nested header message", rewrite(short) { |d| d["h"]["k"] = { "p" => b64("key"), "h" => { "iv" => b64("x") } } } ],
        [ "doubly nested header message", rewrite(short) { |d| d["h"]["k"] = { "p" => b64("key"), "h" => { "n" => { "p" => b64("x") } } } } ],
        [ "headers as array", rewrite(short) { |d| d["h"] = [] } ],
        [ "headers as string", rewrite(short) { |d| d["h"] = "x" } ],
        [ "headers null", rewrite(short) { |d| d["h"] = nil } ],
        [ "null auth tag", rewrite(short) { |d| d["h"]["at"] = nil } ],
        [ "numeric auth tag", rewrite(short) { |d| d["h"]["at"] = 5 } ],
        [ "null key id header", rewrite(short) { |d| d["h"]["i"] = nil } ],
        [ "false key id header", rewrite(short) { |d| d["h"]["i"] = false } ],
        [ "numeric key id header", rewrite(short) { |d| d["h"]["i"] = 5 } ],
        [ "unknown encoding header", rewrite(short) { |d| d["h"]["e"] = b64("NOT-AN-ENCODING") } ],
        # Real strings in encodings Ruby knows but Smartfire never writes: Rails decrypts them,
        # Rust refuses them (fail closed; see AR_FAIL_CLOSED in golden_smartfire.rs).
        [ "Shift_JIS plaintext", value_for_database("Webhook", "signing_secret", "テスト秘密".encode("Shift_JIS")) ],
        [ "ISO-8859-1 plaintext", value_for_database("Webhook", "signing_secret", "café crème".encode("ISO-8859-1")) ],
        [ "compressed flag 0 on compressed data", rewrite(long) { |d| d["h"]["c"] = 0 } ],
        [ "compressed flag as string", rewrite(long) { |d| d["h"]["c"] = b64("yes") } ],
        [ "null payload", rewrite(short) { |d| d["p"] = nil } ],
        [ "empty payload", rewrite(encryptions.find { |e| e["value_hex"].empty? }["ciphertext"]) { |_| } ],
        [ "pretty printed json", JSON.pretty_generate(message_json(short)) ],
        [ "trailing data after json", short + "x" ],
        [ "plaintext (unencrypted legacy value)", "plain-token-value" ],
        [ "json string", JSON.dump("x") ],
        [ "json array", JSON.dump([ 1 ]) ],
        [ "json object without payload", JSON.dump({ "h" => {} }) ],
        [ "empty string", "" ],
        [ "payload of another message", rewrite(short) { |d| d["p"] = message_json(long)["p"] } ]
      ]

      type = GoogleAccount.type_for_attribute("access_token")
      cases.map do |label, ciphertext|
        result = outcome { type.deserialize(ciphertext) }
        entry = { "case" => label, "ciphertext" => ciphertext }
        if result.key?("value")
          value = result["value"]
          entry.merge("expected_hex" => value&.unpack1("H*"), "expected_encoding" => value&.encoding&.name)
        else
          entry.merge("error" => result["error"])
        end
      end
    end

    # --- MessageEncryptor: Calendar::DisconnectCleanupJob credentials ---------------------------

    def message_encryptor_vectors
      account = GoogleAccount.find_by!(user: @david)
      blob = account.cleanup_snapshot
      encryptor = Calendar::DisconnectCleanupJob.credentials_encryptor
      plaintext = encryptor.send(:decrypt, blob)

      other_purpose = encryptor.encrypt_and_sign({ "a" => 1 }, purpose: "something/else")
      no_purpose = encryptor.encrypt_and_sign({ "a" => 1 })
      no_expiry = encryptor.encrypt_and_sign({ "a" => 1 }, purpose: Calendar::DisconnectCleanupJob::CREDENTIALS_PURPOSE)
      rotated = ActiveSupport::MessageEncryptor.new(rotated_key_generator.generate_key(Calendar::DisconnectCleanupJob::CREDENTIALS_PURPOSE, 32))
        .encrypt_and_sign({ "a" => 1 }, purpose: Calendar::DisconnectCleanupJob::CREDENTIALS_PURPOSE)
      marshal = ActiveSupport::MessageEncryptor.new(app.key_generator.generate_key(Calendar::DisconnectCleanupJob::CREDENTIALS_PURPOSE, 32), serializer: :marshal)
        .encrypt_and_sign("marshaled", purpose: Calendar::DisconnectCleanupJob::CREDENTIALS_PURPOSE)
      encrypted, iv, tag = blob.split("--")

      cases = [
        [ "valid", blob, NOW ],
        [ "just before a day", blob, NOW + 1.day - 1.second ],
        [ "at a day", blob, NOW + 1.day ],
        [ "after a day", blob, NOW + 2.days ],
        [ "other purpose", other_purpose, NOW ],
        [ "no purpose", no_purpose, NOW ],
        [ "no expiry", no_expiry, NOW + 10.years ],
        [ "rotated secret", rotated, NOW ],
        [ "marshal payload", marshal, NOW ],
        [ "tampered ciphertext", "#{tamper_first(encrypted)}--#{iv}--#{tag}", NOW ],
        [ "tampered iv", "#{encrypted}--#{tamper_first(iv)}--#{tag}", NOW ],
        [ "tampered auth tag", "#{encrypted}--#{iv}--#{tamper_first(tag)}", NOW ],
        [ "truncated auth tag", "#{encrypted}--#{iv}--#{tag[0, 12]}", NOW ],
        [ "garbage", "garbage", NOW ],
        [ "empty", "", NOW ]
      ]
      verify = cases.map do |label, message, now|
        { "case" => label, "blob" => message, "now" => iso(now),
          "expected" => at(now) { Calendar::DisconnectCleanupJob.decrypt_credentials(message) } }
      end

      {
        "calendar_disconnect_cleanup" => {
          "purpose" => Calendar::DisconnectCleanupJob::CREDENTIALS_PURPOSE,
          "expires_in" => Calendar::DisconnectCleanupJob::CREDENTIALS_EXPIRES_IN.to_i,
          "key_len" => ActiveSupport::MessageEncryptor.key_len,
          "generate" => { "snapshot" => JSON.parse(plaintext).dig("_rails", "data"), "blob" => blob, "plaintext" => plaintext },
          "verify" => verify
        }
      }
    end

    # --- Named message verifiers ----------------------------------------------------------------

    def state_verifiers
      {
        "google_sign_in_state" => Sessions::GoogleController.new.send(:google_sign_in_state_verifier),
        "google_oauth_state" => Google::ConnectionsController.new.send(:state_verifier),
        "github_app_oauth_state" => Github::AppConnectionsController.new.send(:state_verifier),
        "slack_oauth_state" => Slack::OAuthController.new.send(:state_verifier)
      }
    end

    def verifier_vectors
      raw_state = "0123456789abcdef0123456789abcdef"
      states = state_verifiers.map do |name, verifier|
        message = verifier.generate(raw_state)
        cases = [
          [ "valid", message ], [ "tampered", tamper_last(message) ], [ "garbage", "garbage" ], [ "empty", "" ],
          [ "with purpose", verifier.generate(raw_state, purpose: "x") ],
          [ "with expiry", verifier.generate(raw_state, expires_in: 1.minute) ],
          [ "another state verifier", (state_verifiers.values - [ verifier ]).first.generate(raw_state) ],
          [ "non-string state", verifier.generate({ "a" => 1 }) ]
        ]
        { "name" => name, "raw_state" => raw_state, "message" => message,
          "verify" => cases.map { |label, m| { "case" => label, "message" => m, "expected" => verifier.verified(m.to_s) } } }
      end

      url = "https://images.example.com/a/b.png?x=1&y=%3C2%3E&z=<&>"
      signed_path = Embeds::ImageProxy.signed_path(url)
      signed = Embeds::ImageProxy.verifier.generate(url)
      embed_cases = [
        [ "valid", signed ],
        [ "unicode url", Embeds::ImageProxy.verifier.generate("https://images.example.com/ü.png") ],
        [ "http url", Embeds::ImageProxy.verifier.generate("http://example.com/i.png") ],
        [ "javascript url", Embeds::ImageProxy.verifier.generate("javascript:alert(1)") ],
        [ "relative url", Embeds::ImageProxy.verifier.generate("/local.png") ],
        [ "ftp url", Embeds::ImageProxy.verifier.generate("ftp://example.com/i.png") ],
        [ "unparseable url", Embeds::ImageProxy.verifier.generate("http://exa mple.com/") ],
        [ "non-string", Embeds::ImageProxy.verifier.generate(42) ],
        [ "tampered", tamper_last(Embeds::ImageProxy.verifier.generate(url)) ],
        [ "bot_reply verifier", app.message_verifier("bot_reply").generate(url) ]
      ] + [
        "http:foo", "HTTP://EXAMPLE.COM/I.PNG", "http://", "http:///a", "https://u:p@h:99999/p?q r#f", "https://h/p?%zz",
        "https://h/p?%z1", "https://h/p#a b", "https://h/p#%zz", "http://[::1]/i.png", "http://[::1/", "http://1.2.3.4.5/",
        "http://a%zz/", "http://a%41/", "https://h/p\tq", "https://h/a|b", "https://h/?a|b", "https://h:/p", "https://h:8a/p",
        "http://h/[x]", "http://h/?[x]", "https://h/%E2%98%83.png", "mailto:a@b.c", "//h/p.png", "https//h/p.png", " https://h/p"
      ].map { |edge| [ "edge #{edge.inspect}", Embeds::ImageProxy.verifier.generate(edge) ] }

      bot = User.create_bot!(name: "Parity Bot")
      @room.memberships.find_or_create_by!(user: bot)
      token = bot.reply_token_for(@room)
      reply_verifier = User.send(:reply_verifier)
      reply_cases = [
        [ "valid", token, @room.id, NOW ],
        [ "valid with surrounding whitespace", " \t#{token}\n ", @room.id, NOW ],
        [ "valid, room id as string", token, @room.id.to_s, NOW ],
        [ "before expiry", token, @room.id, NOW + 15.minutes - 1.second ],
        [ "at expiry", token, @room.id, NOW + 15.minutes ],
        [ "other room", token, @room.id + 1, NOW ],
        [ "tampered", tamper_last(token), @room.id, NOW ],
        [ "embed_image verifier", Embeds::ImageProxy.verifier.generate({ "bot_id" => bot.id, "room_id" => @room.id }), @room.id, NOW ],
        [ "not a hash", reply_verifier.generate([ bot.id, @room.id ]), @room.id, NOW ],
        [ "unknown bot", reply_verifier.generate({ bot_id: 999, room_id: @room.id }), @room.id, NOW ],
        [ "not a bot", reply_verifier.generate({ bot_id: @david.id, room_id: @room.id }), @room.id, NOW ]
      ]

      {
        "oauth_states" => states,
        "embed_image" => {
          "url" => url, "signed_path" => signed_path, "signed" => signed,
          "verify" => embed_cases.map do |label, m|
            { "case" => label, "message" => m, "verified" => Embeds::ImageProxy.verifier.verified(m),
              "verified_url" => Embeds::ImageProxy.verified_url(m) }
          end
        },
        "bot_reply" => {
          "bot_id" => bot.id, "room_id" => @room.id, "expires_in" => User::Bot::REPLY_URL_EXPIRY.to_i, "token" => token,
          "data_json" => ActiveSupport::JSON.encode(reply_verifier.verified(token)),
          "verify" => reply_cases.map do |label, t, room_id, now|
            { "case" => label, "token" => t, "room_id" => room_id.to_s, "now" => iso(now),
              "data" => at(now) { reply_verifier.verified(t.to_s.strip) },
              "authenticated_bot_id" => at(now) { User.authenticate_bot_reply_token(t, room_id: room_id)&.id } }
          end
        }
      }
    end

    # --- Signed cookies set by Authentication -----------------------------------------------------

    def https_controller(cookies: {})
      env = { "HTTPS" => "on", "rack.url_scheme" => "https", "HTTP_USER_AGENT" => USER_AGENT, "REMOTE_ADDR" => "127.0.0.1" }
      rack_env = Rack::MockRequest.env_for("https://#{HOST}/", "HTTP_COOKIE" => cookie_header(cookies))
      request = ActionDispatch::Request.new(app.env_config.merge(rack_env).merge(env))
      SessionsController.new.tap { |controller| controller.set_request!(request) }
    end

    def written_cookie(controller, name)
      response = Rack::Response.new
      controller.request.cookie_jar.write(response)
      header = Array(response.headers["set-cookie"]).flat_map { |h| h.split("\n") }.find { |h| h.start_with?("#{name}=") }
      [ Rack::Utils.unescape(header[/\A[^=]+=([^;]*)/, 1]), header ]
    end

    def read_signed(name, raw, now)
      at(now) { https_controller(cookies: { name => raw }).request.cookie_jar.signed[name] }
    end

    def cookie_vectors
      device = https_controller
      device_id = device.send(:ensure_device_cookie)
      device_raw, device_header = written_cookie(device, "device_id")
      # A browser that already has one keeps it: no new cookie.
      again = https_controller(cookies: { "device_id" => device_raw })
      again_id = again.send(:ensure_device_cookie)
      again_response = Rack::Response.new
      again.request.cookie_jar.write(again_response)

      remember = https_controller
      remember.send(:remember_two_factor_device!, @david)
      remember_raw, remember_header = written_cookie(remember, "two_factor_remember")
      remember_token = remember.request.cookie_jar.signed[:two_factor_remember]

      verify = [
        [ "device_id valid", "device_id", device_raw, NOW ],
        [ "device_id in 19 years", "device_id", device_raw, NOW + 19.years ],
        [ "device_id after 20 years", "device_id", device_raw, NOW + 20.years + 1.second ],
        [ "device_id under the remember name", "two_factor_remember", device_raw, NOW ],
        [ "device_id tampered", "device_id", tamper_last(device_raw), NOW ],
        [ "remember valid", "two_factor_remember", remember_raw, NOW ],
        [ "remember before 30 days", "two_factor_remember", remember_raw, NOW + 30.days - 1.second ],
        [ "remember at 30 days", "two_factor_remember", remember_raw, NOW + 30.days ],
        [ "remember under the device_id name", "device_id", remember_raw, NOW ],
        [ "remember under the session_token name", "session_token", remember_raw, NOW ],
        [ "remember tampered", "two_factor_remember", tamper_first(remember_raw), NOW ]
      ].map { |label, name, raw, now| { "case" => label, "name" => name, "raw" => raw, "now" => iso(now), "expected" => read_signed(name, raw, now) } }

      {
        "device_id" => { "value" => device_id, "raw" => device_raw, "set_cookie" => device_header, "expires_at" => iso(NOW + 20.years),
                         "existing_value" => again_id, "existing_rewrites" => Array(again_response.headers["set-cookie"]).any? },
        "two_factor_remember" => { "name" => Authentication::TWO_FACTOR_REMEMBER_COOKIE.to_s, "value" => remember_token,
                                   "raw" => remember_raw, "set_cookie" => remember_header, "expires_at" => iso(NOW + TwoFactorRememberedDevice::REMEMBER_FOR),
                                   "digest_stored" => TwoFactorRememberedDevice.last.token_digest == TwoFactorRememberedDevice.digest(remember_token) },
        "verify" => verify
      }
    end

    # --- Signed ids through our models ------------------------------------------------------------

    def signed_id_vectors
      transfer = @david.transfer_id
      avatar = @david.avatar_token
      cases = [
        [ "transfer valid", "transfer", transfer, NOW ],
        [ "transfer just before 4 hours", "transfer", transfer, NOW + 4.hours - 1.second ],
        [ "transfer at 4 hours", "transfer", transfer, NOW + 4.hours ],
        [ "avatar token as transfer id", "transfer", avatar, NOW ],
        [ "avatar valid", "avatar", avatar, NOW ],
        [ "avatar far future", "avatar", avatar, NOW + 30.years ],
        [ "transfer id as avatar token", "avatar", transfer, NOW ],
        [ "room avatar as user avatar", "avatar", @room.signed_id(purpose: :avatar), NOW ],
        [ "tampered avatar", "avatar", tamper_last(avatar), NOW ]
      ].map do |label, kind, signed, now|
        result = at(now) do
          outcome { kind == "transfer" ? User.find_by_transfer_id(signed)&.id : User.from_avatar_token(signed).id }
        end
        { "case" => label, "kind" => kind, "signed_id" => signed, "now" => iso(now),
          "expected_id" => result["value"], "error" => result["error"] }
      end

      { "user_id" => @david.id, "transfer_expires_in" => User::Transferable::TRANSFER_LINK_EXPIRY_DURATION.to_i,
        "transfer_id" => transfer, "avatar_token" => avatar, "verify" => cases }
    end

    # --- LiveKit JWTs (HS256) ---------------------------------------------------------------------

    Grant = Struct.new(:room_name, :identity, :server_muted, :stage_role, :id) do
      def server_muted? = server_muted
    end
    Room = Struct.new(:stage) do
      def stage? = stage
    end
    Person = Struct.new(:name)

    def huddle_token(name:, identity:, room_name:, stage: false, stage_role: nil, muted: false)
      huddle = Huddle.allocate
      huddle.instance_variable_set(:@grant, Grant.new(room_name, identity, muted, stage_role, 1))
      huddle.instance_variable_set(:@room, Room.new(stage))
      huddle.instance_variable_set(:@user, Person.new(name))
      token = huddle.token
      claims, header = JWT.decode(token, nil, false)
      { "token" => token, "header" => header, "claims" => claims, "claims_json" => Base64.urlsafe_decode64(token.split(".")[1]).force_encoding("UTF-8") }
    end

    # Signs by hand: JWT.encode refuses some of the malformed claims these cases need.
    def livekit_encode(claims, secret: LIVEKIT_API_SECRET, algorithm: "HS256", header: {})
      header = { "alg" => algorithm }.merge(header.transform_keys(&:to_s))
      signing_input = [ Base64.urlsafe_encode64(JSON.generate(header), padding: false), Base64.urlsafe_encode64(JSON.generate(claims), padding: false) ].join(".")
      digest = { "HS256" => "SHA256", "HS512" => "SHA512" }.fetch(algorithm)
      "#{signing_input}.#{Base64.urlsafe_encode64(OpenSSL::HMAC.digest(digest, secret, signing_input), padding: false)}"
    end

    def livekit_vectors
      room_name = Huddle.room_name(42)
      tokens = [
        [ "publisher", huddle_token(name: "David", identity: "campfire-participant-abc", room_name: room_name) ],
        [ "stage listener", huddle_token(name: "Jason", identity: "campfire-participant-def", room_name: room_name, stage: true, stage_role: "listener") ],
        [ "server muted", huddle_token(name: "Muted", identity: "campfire-participant-ghi", room_name: room_name, muted: true) ],
        [ "unicode and quotes in name", huddle_token(name: "Zoë \"Z\" </script> ☃   tab\t ctl\u0001\u001f del\u007f \\", identity: "campfire-participant-jkl", room_name: room_name) ]
      ]
      admin = Huddle::RoomService.new.send(:admin_token, { roomAdmin: true, room: room_name })
      admin_claims, admin_header = JWT.decode(admin, nil, false)

      publisher = tokens.first[1]["claims"]
      now = NOW.to_i
      base = publisher.merge("exp" => now + 120, "iat" => now, "nbf" => now - 5)
      listener_video = { "room" => room_name, "roomJoin" => true, "canSubscribe" => true }
      cases = tokens.map { |label, t| [ "minted #{label}", t["token"], NOW ] } + [
        [ "admin token (no sub)", admin, NOW ],
        [ "just before exp", tokens.first[1]["token"], NOW + 119.seconds ],
        [ "at exp", tokens.first[1]["token"], NOW + 120.seconds ],
        [ "nbf in the future", livekit_encode(base.merge("nbf" => now + 10)), NOW ],
        [ "nbf 5s ago", livekit_encode(base), NOW ],
        [ "wrong secret", livekit_encode(base, secret: "another-secret-0123456789abcdef0123456789"), NOW ],
        [ "wrong issuer", livekit_encode(base.merge("iss" => "other")), NOW ],
        [ "missing iss", livekit_encode(base.except("iss")), NOW ],
        [ "missing exp", livekit_encode(base.except("exp")), NOW ],
        [ "missing nbf", livekit_encode(base.except("nbf")), NOW ],
        [ "missing sub", livekit_encode(base.except("sub")), NOW ],
        [ "missing video", livekit_encode(base.except("video")), NOW ],
        [ "float exp", livekit_encode(base.merge("exp" => now + 120.5)), NOW ],
        [ "string exp", livekit_encode(base.merge("exp" => (now + 120).to_s)), NOW ],
        [ "string nbf", livekit_encode(base.merge("nbf" => (now - 5).to_s)), NOW ],
        [ "boolean exp", livekit_encode(base.merge("exp" => true)), NOW ],
        [ "alg none", [ Base64.urlsafe_encode64('{"alg":"none"}', padding: false), Base64.urlsafe_encode64(JSON.generate(base), padding: false), "" ].join(".") ],
        [ "alg none, two segments", [ Base64.urlsafe_encode64('{"alg":"none"}', padding: false), Base64.urlsafe_encode64(JSON.generate(base), padding: false) ].join(".") ],
        [ "HS512", livekit_encode(base, algorithm: "HS512"), NOW ],
        [ "lowercase alg header", resign_with_header(base, { "alg" => "hs256" }), NOW ],
        [ "typ header", livekit_encode(base, header: { typ: "JWT" }), NOW ],
        [ "b64 false header", resign_with_header(base, { "alg" => "HS256", "b64" => false, "crit" => [ "b64" ] }), NOW ],
        [ "header not an object", resign_with_header(base, [ "HS256" ]), NOW ],
        [ "tampered payload", tamper_segment(tokens.first[1]["token"], 1), NOW ],
        [ "tampered signature", tamper_segment(tokens.first[1]["token"], 2), NOW ],
        [ "padded segments", tokens.first[1]["token"].split(".").map { |s| s + "=" * ((4 - s.length % 4) % 4) }.join("."), NOW ],
        [ "standard base64 alphabet", tokens.first[1]["token"].tr("-_", "+/"), NOW ],
        [ "four segments", tokens.first[1]["token"] + ".x", NOW ],
        [ "empty", "", NOW ],
        [ "garbage", "not.a.jwt", NOW ],
        [ "payload is an array", resign_payload("[1]"), NOW ],
        [ "listener refreshed by LiveKit (no canPublish, no sources)", livekit_encode(base.merge("video" => listener_video)), NOW ],
        [ "listener with canPublish false and empty sources", livekit_encode(base.merge("video" => listener_video.merge("canPublish" => false, "canPublishSources" => []))), NOW ],
        [ "publisher with sources in another order", livekit_encode(base.merge("video" => base["video"].merge("canPublishSources" => base["video"]["canPublishSources"].reverse))), NOW ],
        [ "publisher missing a source", livekit_encode(base.merge("video" => base["video"].merge("canPublishSources" => %w[ microphone camera ]))), NOW ],
        [ "listener with sources", livekit_encode(base.merge("video" => listener_video.merge("canPublishSources" => %w[ microphone ]))), NOW ],
        [ "canPublish string", livekit_encode(base.merge("video" => base["video"].merge("canPublish" => "true"))), NOW ],
        [ "sources not strings", livekit_encode(base.merge("video" => listener_video.merge("canPublishSources" => [ 1 ]))), NOW ],
        [ "forbidden roomAdmin", livekit_encode(base.merge("video" => base["video"].merge("roomAdmin" => true))), NOW ],
        [ "forbidden roomAdmin false", livekit_encode(base.merge("video" => base["video"].merge("roomAdmin" => false))), NOW ],
        [ "forbidden hidden as string", livekit_encode(base.merge("video" => base["video"].merge("hidden" => "no"))), NOW ],
        [ "unknown video key", livekit_encode(base.merge("video" => base["video"].merge("extra" => true))), NOW ],
        [ "roomJoin false", livekit_encode(base.merge("video" => base["video"].merge("roomJoin" => false))), NOW ],
        [ "canSubscribe missing", livekit_encode(base.merge("video" => base["video"].except("canSubscribe"))), NOW ],
        [ "empty room", livekit_encode(base.merge("video" => base["video"].merge("room" => ""))), NOW ],
        [ "blank sub", livekit_encode(base.merge("sub" => "")), NOW ],
        [ "numeric sub", livekit_encode(base.merge("sub" => 5)), NOW ],
        [ "video not an object", livekit_encode(base.merge("video" => "x")), NOW ],
        [ "missing room", livekit_encode(base.merge("video" => base["video"].except("room"))), NOW ],
        [ "extra top-level claims", livekit_encode(base.merge("metadata" => "x", "kind" => "standard")), NOW ]
      ]

      verify = cases.map do |label, token, now|
        now ||= NOW
        result = at(now) { outcome { Huddle::TokenVerifier.new(token).coordinates } }
        { "case" => label, "token" => token, "now" => iso(now),
          "expected" => result["value"]&.transform_keys(&:to_s), "error" => result["error"] }
      end

      {
        "api_key" => LIVEKIT_API_KEY, "api_secret" => LIVEKIT_API_SECRET,
        "token_ttl" => Huddle::TOKEN_TTL.to_i, "admin_token_ttl" => Huddle::RoomService::TOKEN_TTL.to_i,
        "publish_sources" => Huddle::PUBLISH_SOURCES, "room_name" => room_name, "room_id" => 42,
        "participant_tokens" => tokens.map { |label, t| t.merge("case" => label) },
        "admin_token" => { "token" => admin, "header" => admin_header, "claims" => admin_claims,
                           "claims_json" => Base64.urlsafe_decode64(admin.split(".")[1]).force_encoding("UTF-8"), "grant" => { "roomAdmin" => true, "room" => room_name } },
        "verify" => verify
      }
    end

    def resign_with_header(payload, header)
      signing_input = [ Base64.urlsafe_encode64(JSON.generate(header), padding: false), Base64.urlsafe_encode64(JSON.generate(payload), padding: false) ].join(".")
      "#{signing_input}.#{Base64.urlsafe_encode64(OpenSSL::HMAC.digest("SHA256", LIVEKIT_API_SECRET, signing_input), padding: false)}"
    end

    def resign_payload(payload_json)
      signing_input = [ Base64.urlsafe_encode64('{"alg":"HS256"}', padding: false), Base64.urlsafe_encode64(payload_json, padding: false) ].join(".")
      "#{signing_input}.#{Base64.urlsafe_encode64(OpenSSL::HMAC.digest("SHA256", LIVEKIT_API_SECRET, signing_input), padding: false)}"
    end

    def tamper_segment(token, index)
      parts = token.split(".")
      parts[index] = tamper_first(parts[index])
      parts.join(".")
    end

    # --- Google ID tokens (RS256 against Google's JWKS) -------------------------------------------

    module FakeGoogleKeys
      def start(host, *args, **kwargs, &block)
        return super unless host == "www.googleapis.com"

        response = Net::HTTPOK.new("1.1", "200", "OK")
        response.instance_variable_set(:@body, Thread.current[:rails_compat_jwks])
        response.instance_variable_set(:@read, true)
        http = Object.new
        http.define_singleton_method(:get) { |_path| response }
        block.call(http)
      end
    end
    Net::HTTP.singleton_class.prepend(FakeGoogleKeys)

    def jwk_for(key, kid)
      { "kty" => "RSA", "alg" => "RS256", "use" => "sig", "kid" => kid,
        "n" => Base64.urlsafe_encode64(key.n.to_s(2), padding: false), "e" => Base64.urlsafe_encode64(key.e.to_s(2), padding: false) }
    end

    def with_jwks(jwks_json)
      Thread.current[:rails_compat_jwks] = jwks_json
      Google::SignIn::KeyStore.clear!
      yield
    ensure
      Thread.current[:rails_compat_jwks] = nil
      Google::SignIn::KeyStore.clear!
    end

    def google_vectors
      key = OpenSSL::PKey::RSA.generate(2048)
      other_key = OpenSSL::PKey::RSA.generate(2048)
      small_key = OpenSSL::PKey::RSA.generate(1024)
      jwks = { "keys" => [ jwk_for(key, "kid-1"), jwk_for(other_key, "kid-2"), jwk_for(small_key, "kid-small") ] }
      jwks_json = JSON.generate(jwks)

      now = NOW.to_i
      claims = {
        "iss" => "https://accounts.google.com", "azp" => GOOGLE_CLIENT_ID, "aud" => GOOGLE_CLIENT_ID,
        "sub" => "110169484474386276334", "hd" => "example.com", "email" => "david@example.com",
        "email_verified" => true, "at_hash" => "HK6E_P6Dh8Y93mRNtsDB1Q", "nonce" => "nonce-123",
        "auth_time" => now - 60, "iat" => now - 10, "exp" => now + 3590
      }
      sign = ->(payload, kid: "kid-1", signing_key: key) { rs_resign({ "alg" => "RS256", "kid" => kid }.compact, payload, signing_key) }

      cases = [
        [ "valid", sign.(claims), {} ],
        [ "valid, other key", sign.(claims, kid: "kid-2", signing_key: other_key), {} ],
        [ "valid, issuer without scheme", sign.(claims.merge("iss" => "accounts.google.com")), {} ],
        [ "valid, fresh re-auth", sign.(claims), { "max_auth_age" => 300 } ],
        [ "stale re-auth", sign.(claims.merge("auth_time" => now - 400)), { "max_auth_age" => 300 } ],
        [ "re-auth without auth_time", sign.(claims.except("auth_time")), { "max_auth_age" => 300 } ],
        [ "re-auth with string auth_time", sign.(claims.merge("auth_time" => (now - 10).to_s)), { "max_auth_age" => 300 } ],
        [ "re-auth at the skew boundary", sign.(claims.merge("auth_time" => now - 330)), { "max_auth_age" => 300 } ],
        [ "re-auth just inside the skew", sign.(claims.merge("auth_time" => now - 329)), { "max_auth_age" => 300 } ],
        [ "signed by a key under another kid", sign.(claims, kid: "kid-2"), {} ],
        [ "unknown kid", sign.(claims, kid: "kid-9"), {} ],
        [ "missing kid", sign.(claims, kid: nil), {} ],
        [ "blank kid", sign.(claims, kid: " "), {} ],
        [ "numeric kid", rs_resign({ "alg" => "RS256", "kid" => 1 }, claims, key), {} ],
        # ruby-jwt refuses to sign with a key under 2048 bits, but verifies with one.
        [ "1024-bit key", rs_resign({ "alg" => "RS256", "kid" => "kid-small" }, claims, small_key), {} ],
        [ "HS256 with the public key as secret", JWT.encode(claims, key.public_key.to_pem, "HS256", { kid: "kid-1" }), {} ],
        [ "alg none", [ Base64.urlsafe_encode64('{"alg":"none","kid":"kid-1"}', padding: false), Base64.urlsafe_encode64(JSON.generate(claims), padding: false), "" ].join("."), {} ],
        [ "lowercase alg", rs_resign({ "alg" => "rs256", "kid" => "kid-1" }, claims, key), {} ],
        [ "wrong issuer", sign.(claims.merge("iss" => "https://evil.example.com")), {} ],
        [ "wrong audience", sign.(claims.merge("aud" => "other-client", "azp" => nil).compact), {} ],
        [ "audience list with azp", sign.(claims.merge("aud" => [ GOOGLE_CLIENT_ID, "other" ])), {} ],
        [ "audience list without azp", sign.(claims.merge("aud" => [ GOOGLE_CLIENT_ID, "other" ]).except("azp")), {} ],
        [ "single audience, no azp", sign.(claims.except("azp")), {} ],
        [ "azp mismatch", sign.(claims.merge("azp" => "other")), {} ],
        [ "nested audience array", sign.(claims.merge("aud" => [ [ GOOGLE_CLIENT_ID ] ]).except("azp")), {} ],
        [ "numeric audience", sign.(claims.merge("aud" => 5)), {} ],
        # `Array(aud).flatten` flattens arrays only: a hash inside the list stays one opaque
        # element (its #to_s is its inspect), while a top-level hash becomes its [key, value] pairs.
        [ "audience hash in a list", sign.(claims.merge("aud" => [ { "nested" => GOOGLE_CLIENT_ID } ])), {} ],
        [ "audience hash in a list, no azp", sign.(claims.merge("aud" => [ { "nested" => GOOGLE_CLIENT_ID } ]).except("azp")), {} ],
        [ "audience hash in a nested list", sign.(claims.merge("aud" => [ [ { GOOGLE_CLIENT_ID => GOOGLE_CLIENT_ID } ] ])), {} ],
        [ "audience list with a hash beside the client id", sign.(claims.merge("aud" => [ GOOGLE_CLIENT_ID, { "a" => "b" } ])), {} ],
        [ "audience list with a hash beside the client id, no azp", sign.(claims.merge("aud" => [ GOOGLE_CLIENT_ID, { "a" => "b" } ]).except("azp")), {} ],
        [ "audience hash keyed by the client id", sign.(claims.merge("aud" => { GOOGLE_CLIENT_ID => "x" })), {} ],
        [ "audience hash valued with the client id", sign.(claims.merge("aud" => { "x" => GOOGLE_CLIENT_ID })), {} ],
        [ "audience hash valued with the client id, no azp", sign.(claims.merge("aud" => { "x" => GOOGLE_CLIENT_ID }).except("azp")), {} ],
        [ "audience hash valued with a nested list", sign.(claims.merge("aud" => { "x" => [ [ GOOGLE_CLIENT_ID ] ] })), {} ],
        [ "audience hash valued with a hash", sign.(claims.merge("aud" => { "x" => { "y" => GOOGLE_CLIENT_ID } })), {} ],
        [ "audience hash keyed by the client id with a null value, no azp", sign.(claims.merge("aud" => { GOOGLE_CLIENT_ID => nil }).except("azp")), {} ],
        [ "audience list with nulls, no azp", sign.(claims.merge("aud" => [ nil, [ GOOGLE_CLIENT_ID, nil ] ]).except("azp")), {} ],
        [ "empty audience hash", sign.(claims.merge("aud" => {})), {} ],
        [ "expired", sign.(claims.merge("exp" => now - 1)), {} ],
        [ "expires now", sign.(claims.merge("exp" => now)), {} ],
        [ "expires in a second", sign.(claims.merge("exp" => now + 1)), {} ],
        [ "missing exp", sign.(claims.except("exp")), {} ],
        [ "string exp", sign.(claims.merge("exp" => (now + 100).to_s)), {} ],
        [ "float exp", sign.(claims.merge("exp" => now + 0.5)), {} ],
        [ "nbf in the future", sign.(claims.merge("nbf" => now + 60)), {} ],
        [ "nbf now", sign.(claims.merge("nbf" => now)), {} ],
        [ "iat in the future", sign.(claims.merge("iat" => now + 600)), {} ],
        [ "missing sub", sign.(claims.except("sub")), {} ],
        [ "blank sub", sign.(claims.merge("sub" => " ")), {} ],
        [ "missing email", sign.(claims.except("email")), {} ],
        [ "unverified email", sign.(claims.merge("email_verified" => false)), {} ],
        [ "email_verified as string", sign.(claims.merge("email_verified" => "true")), {} ],
        [ "wrong nonce", sign.(claims.merge("nonce" => "nonce-124")), {} ],
        [ "missing nonce", sign.(claims.except("nonce")), {} ],
        [ "numeric nonce", sign.(claims.merge("nonce" => 5)), {} ],
        [ "no hd", sign.(claims.except("hd")), {} ],
        [ "hd not allowed", sign.(claims.merge("hd" => "other.com", "email" => "david@other.com")), {} ],
        [ "hd allowed, email elsewhere", sign.(claims.merge("email" => "david@gmail.com")), {} ],
        [ "hd and email in mixed case with spaces", sign.(claims.merge("hd" => " Example.COM ", "email" => "David@EXAMPLE.com ")), {} ],
        [ "second allowed domain", sign.(claims.merge("hd" => "smartdata.test", "email" => "x@smartdata.test")), {} ],
        [ "subdomain not listed", sign.(claims.merge("hd" => "a.example.com", "email" => "x@a.example.com")), {} ],
        [ "invalid listed domain", sign.(claims.merge("hd" => "-bad.test", "email" => "x@-bad.test")), {} ],
        [ "tampered payload", tamper_segment(sign.(claims), 1), {} ],
        [ "tampered signature", tamper_segment(sign.(claims), 2), {} ],
        [ "payload not an object", rs_resign({ "alg" => "RS256", "kid" => "kid-1" }, [ 1 ], key), {} ],
        [ "empty token", "", {} ],
        [ "blank nonce expected", sign.(claims), { "nonce" => "" } ],
        [ "two segments", sign.(claims).split(".")[0, 2].join("."), {} ],
        [ "garbage", "a.b.c", {} ]
      ]

      verify = with_jwks(jwks_json) do
        cases.map do |label, token, options|
          nonce = options.fetch("nonce", "nonce-123")
          max_auth_age = options["max_auth_age"]&.seconds
          result = begin
            { "claims" => Google::SignIn::IdTokenVerifier.verify!(token, nonce: nonce, max_auth_age: max_auth_age) }
          rescue Google::SignIn::Rejected => error
            { "rejected" => error.reason.to_s }
          rescue Google::SignIn::Unavailable
            { "unavailable" => true }
          rescue StandardError => error
            { "error" => error.class.name }
          end
          { "case" => label, "token" => token, "nonce" => nonce, "max_auth_age" => options["max_auth_age"] }.merge(result)
        end
      end

      jwks_cases = [
        [ "google shaped", jwks_json ],
        [ "mixed entries", JSON.generate({ "keys" => [
          jwk_for(key, "good"), jwk_for(key, "").merge("kid" => ""), { "kty" => "EC", "kid" => "ec", "x" => "a", "y" => "b" },
          jwk_for(key, "no-n").except("n"), jwk_for(key, "numeric-e").merge("e" => 65537), jwk_for(key, "bad-n").merge("n" => "!!!"),
          jwk_for(key, "padded").merge("n" => Base64.urlsafe_encode64(key.n.to_s(2))), "not a hash", jwk_for(other_key, "good")
        ] }) ],
        [ "no keys", JSON.generate({ "keys" => [] }) ],
        [ "keys not an array", JSON.generate({ "keys" => {} }) ],
        [ "not an object", JSON.generate([ 1 ]) ],
        [ "not json", "<html>" ]
      ].map do |label, body|
        Thread.current[:rails_compat_jwks] = body
        Google::SignIn::KeyStore.clear!
        result = begin
          keys = Google::SignIn::KeyStore.send(:fetch_keys!)
          { "kids" => keys.keys, "moduli" => keys.transform_values { |k| k.n.to_s(16).downcase } }
        rescue Google::SignIn::Unavailable
          { "unavailable" => true }
        rescue StandardError => error
          { "error" => error.class.name }
        end
        { "case" => label, "body" => body }.merge(result)
      ensure
        Thread.current[:rails_compat_jwks] = nil
        Google::SignIn::KeyStore.clear!
      end

      {
        "client_id" => GOOGLE_CLIENT_ID, "sign_in_domains_env" => GOOGLE_SIGN_IN_DOMAINS,
        "allowed_domains" => Google::SignIn.allowed_domains, "issuers" => Google::SignIn::ISSUERS,
        "clock_skew" => Google::SignIn::CLOCK_SKEW.to_i, "fresh_login_max_auth_age" => Google::SignIn::FRESH_LOGIN_MAX_AUTH_AGE.to_i,
        "jwks" => jwks_json, "verify" => verify, "jwks_parsing" => jwks_cases,
        "allowed_domain_parsing" => [ "", " a.com ", "A.COM,a.com", "a,b.c", "x_y.com, ok.io", "a-.com,-a.com,a-b.com", "é.com,xn--e-9ga.com", "a..com,.a.com,a.com." ].map do |raw|
          ENV["GOOGLE_SIGN_IN_DOMAINS"] = raw
          { "env" => raw, "domains" => Google::SignIn.allowed_domains }
        ensure
          ENV["GOOGLE_SIGN_IN_DOMAINS"] = GOOGLE_SIGN_IN_DOMAINS
        end
      }
    end

    def rs_resign(header, payload, key)
      signing_input = [ Base64.urlsafe_encode64(JSON.generate(header), padding: false), Base64.urlsafe_encode64(JSON.generate(payload), padding: false) ].join(".")
      "#{signing_input}.#{Base64.urlsafe_encode64(key.sign("SHA256", signing_input), padding: false)}"
    end

    # --- Webhook signatures -----------------------------------------------------------------------

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
    Net::HTTP.singleton_class.prepend(CaptureWebhookPost)

    module ResolveAnything
      def resolve(host) = "93.184.216.34"
    end

    def webhook_vectors
      RestrictedHTTP::PrivateNetworkGuard.singleton_class.prepend(ResolveAnything)
      webhook = Webhook.new(url: "https://hooks.example.com/deliver")
      secret = SecureRandom.hex(32)
      bodies = [ %({"user":{"id":1,"name":"David"},"message":{"body":{"plain":"hi ☃"}}}), "", "{}", "x" * 5000 ]
      smartfire = bodies.each_with_index.map do |body, index|
        at(NOW + index.seconds) do
          webhook.post_payload(body, secret: secret)
          request = Thread.current[:rails_compat_webhook_request]
          { "secret" => secret, "body" => body, "timestamp" => request[Webhook::TIMESTAMP_HEADER],
            "signature" => request[Webhook::SIGNATURE_HEADER], "content_type" => request["Content-Type"] }
        end
      end
      unsigned = at(NOW) do
        webhook.post_payload("{}", secret: nil)
        request = Thread.current[:rails_compat_webhook_request]
        { "timestamp" => request[Webhook::TIMESTAMP_HEADER], "signature" => request[Webhook::SIGNATURE_HEADER] }
      end

      body = %({"action":"opened","number":1,"repository":{"full_name":"Smart-Data-Ohio/Smartfire"}})
      valid = "sha256=#{OpenSSL::HMAC.hexdigest("SHA256", GITHUB_WEBHOOK_SECRET, body)}"
      github_cases = [
        [ "valid", valid, body ],
        [ "valid, empty body", "sha256=#{OpenSSL::HMAC.hexdigest("SHA256", GITHUB_WEBHOOK_SECRET, "")}", "" ],
        [ "uppercase hex", "sha256=#{valid.delete_prefix("sha256=").upcase}", body ],
        [ "sha1 prefix", valid.sub("sha256=", "sha1="), body ],
        [ "no prefix", valid.delete_prefix("sha256="), body ],
        [ "tampered", tamper_last(valid), body ],
        [ "truncated", valid[0..-2], body ],
        [ "trailing whitespace", valid + " ", body ],
        [ "body changed", valid, body + " " ],
        [ "wrong secret", "sha256=#{OpenSSL::HMAC.hexdigest("SHA256", "other", body)}", body ],
        [ "missing", nil, body ],
        [ "empty", "", body ]
      ].map do |label, signature, raw_body|
        env = Rack::MockRequest.env_for("http://#{HOST}/github/webhooks", method: "POST", input: raw_body)
        env["HTTP_X_HUB_SIGNATURE_256"] = signature if signature
        controller = Github::WebhooksController.new.tap { |c| c.set_request!(ActionDispatch::Request.new(app.env_config.merge(env))) }
        { "case" => label, "signature" => signature, "body" => raw_body, "expected" => controller.send(:valid_signature?, GITHUB_WEBHOOK_SECRET) }
      end

      {
        "signature_header" => Webhook::SIGNATURE_HEADER, "timestamp_header" => Webhook::TIMESTAMP_HEADER,
        "smartfire" => smartfire, "unsigned" => unsigned,
        "github" => { "secret" => GITHUB_WEBHOOK_SECRET, "verify" => github_cases }
      }
    end
end

File.write(File.join(ENV.fetch("VECTORS_DIR"), "rails_compat_smartfire.json"), JSON.pretty_generate(RailsCompatSmartfireVectors.new.generate) + "\n")
puts "Wrote #{File.join(ENV.fetch("VECTORS_DIR"), "rails_compat_smartfire.json")}"
