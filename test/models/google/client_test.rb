require "test_helper"

class Google::ClientTest < ActiveSupport::TestCase
  include GoogleCalendarTestHelper

  setup do
    @account = connect_google!(users(:david))
    @client = Google::Client.new(@account)
  end

  test "configured? requires both client id and secret" do
    assert Google::Client.configured?

    disconnect_google_env!

    assert_not Google::Client.configured?
  end

  test "authorize_url carries the calendar scope, offline access, and state" do
    url = Google::Client.authorize_url(redirect_uri: "http://test.host/google/callback", state: "signed-state")
    query = Rack::Utils.parse_query(URI(url).query)

    assert_equal "https", URI(url).scheme
    assert_equal "test-client-id", query["client_id"]
    assert_equal "http://test.host/google/callback", query["redirect_uri"]
    assert_equal "code", query["response_type"]
    assert_equal "openid email https://www.googleapis.com/auth/calendar.events", query["scope"]
    assert_equal "offline", query["access_type"]
    assert_equal "consent", query["prompt"]
    assert_equal "signed-state", query["state"]
  end

  test "refreshes an expired access token before calling" do
    @account.update!(access_token_expires_at: 1.hour.ago)
    stub_google_token_refresh
    insert = stub_google_event_insert(body: { "id" => "abc123" })

    response = @client.insert_event({ "summary" => "Party" })

    assert_equal "abc123", response["id"]
    assert_requested :post, GOOGLE_TOKEN_URL
    assert_requested :post, GOOGLE_EVENTS_URL, headers: { "Authorization" => "Bearer refreshed-access-token" }
    assert_equal "refreshed-access-token", @account.reload.access_token
    assert @account.access_token_expires_at > 30.minutes.from_now
    assert_requested insert
  end

  test "retries once after a 401 cured by a refresh" do
    stub_google_token_refresh
    stub_request(:post, GOOGLE_EVENTS_URL)
      .to_return({ status: 401 }, { status: 200, body: {}.to_json })

    @client.insert_event({ "summary" => "Party" })

    assert_requested :post, GOOGLE_EVENTS_URL, times: 2
    assert_requested :post, GOOGLE_TOKEN_URL, times: 1
  end

  test "a 401 that survives refresh raises Unauthorized" do
    stub_google_token_refresh
    stub_request(:post, GOOGLE_EVENTS_URL).to_return(status: 401)

    assert_raises(Google::Client::Unauthorized) { @client.insert_event({}) }
  end

  test "invalid_grant marks the account disconnected and raises Unauthorized" do
    @account.update!(access_token_expires_at: 1.hour.ago)
    stub_google_token_invalid_grant

    error = assert_raises(Google::Client::Unauthorized) { @client.insert_event({}) }

    assert_equal "Google rejected the connection", @account.reload.disconnected_reason
    assert_not_predicate @account, :connected?
    assert_not_includes error.message, @account.refresh_token.to_s
  end

  test "a failed refresh without invalid_grant raises Error and keeps the connection" do
    @account.update!(access_token_expires_at: 1.hour.ago)
    stub_request(:post, GOOGLE_TOKEN_URL).to_return(status: 500, body: "boom")

    assert_raises(Google::Client::Error) { @client.insert_event({}) }
    assert_nil @account.reload.disconnected_reason
  end

  test "404 maps to NotFound" do
    stub_google_event_delete("missing-id", status: 404)

    assert_raises(Google::Client::NotFound) { @client.delete_event("missing-id") }
  end

  test "409 maps to Conflict" do
    stub_google_event_insert(status: 409, body: { "error" => { "code" => 409 } })

    assert_raises(Google::Client::Conflict) { @client.insert_event({}) }
  end

  test "other API failures map to Error" do
    stub_google_event_delete("some-id", status: 500)

    assert_raises(Google::Client::Error) { @client.delete_event("some-id") }
  end

  test "a timeout maps to Unavailable" do
    stub_request(:post, GOOGLE_EVENTS_URL).to_timeout

    error = assert_raises(Google::Client::Unavailable) { @client.insert_event({}) }

    assert Google::Client::Unavailable < Google::Client::Error
    assert_equal "Google Calendar request failed (Net::OpenTimeout)", error.message
  end

  test "a malformed response body maps to Unavailable" do
    stub_request(:post, GOOGLE_EVENTS_URL).to_return(status: 200, body: "{oops")

    error = assert_raises(Google::Client::Unavailable) { @client.insert_event({}) }

    assert_equal "Google Calendar request failed (JSON::ParserError)", error.message
  end

  test "email_from_id_token returns the verified email" do
    assert_equal "david@gmail.test", Google::Client.email_from_id_token(google_id_token)
  end

  test "email_from_id_token accepts the short issuer" do
    assert_equal "david@gmail.test",
      Google::Client.email_from_id_token(google_id_token(iss: "accounts.google.com"))
  end

  test "email_from_id_token rejects a missing or malformed token" do
    assert_raises(Google::Client::Error) { Google::Client.email_from_id_token(nil) }
    assert_raises(Google::Client::Error) { Google::Client.email_from_id_token("") }
    assert_raises(Google::Client::Error) { Google::Client.email_from_id_token("not-a-jwt") }
  end

  test "email_from_id_token rejects a wrong issuer, audience, expiry, or missing email" do
    assert_raises(Google::Client::Error) do
      Google::Client.email_from_id_token(google_id_token(iss: "https://evil.test"))
    end
    assert_raises(Google::Client::Error) do
      Google::Client.email_from_id_token(google_id_token(aud: "other-client-id"))
    end
    assert_raises(Google::Client::Error) do
      Google::Client.email_from_id_token(google_id_token(exp: 1.hour.ago.to_i))
    end
    assert_raises(Google::Client::Error) do
      Google::Client.email_from_id_token(google_id_token(email: nil))
    end
  end

  test "exchange_code returns the token response" do
    stub_google_code_exchange

    tokens = Google::Client.exchange_code(code: "auth-code", redirect_uri: "http://test.host/google/callback")

    assert_equal "new-access-token", tokens["access_token"]
    assert_equal "new-refresh-token", tokens["refresh_token"]
    assert_predicate tokens["id_token"], :present?
    assert_requested :post, GOOGLE_TOKEN_URL, body: hash_including({ "code" => "auth-code", "grant_type" => "authorization_code" })
  end

  test "a failed code exchange raises Error" do
    stub_request(:post, GOOGLE_TOKEN_URL).to_return(status: 400, body: { error: "invalid_grant" }.to_json)

    assert_raises(Google::Client::Error) { Google::Client.exchange_code(code: "bad", redirect_uri: "http://test.host/x") }
  end

  test "authorize_url with drive requests Calendar plus the per-file Drive scope" do
    url = Google::Client.authorize_url(redirect_uri: "http://test.host/google/callback", state: "signed-state", drive: true)
    query = Rack::Utils.parse_query(URI(url).query)

    assert_equal "openid email https://www.googleapis.com/auth/calendar.events https://www.googleapis.com/auth/drive.file", query["scope"]
    assert_not_includes query.keys, "include_granted_scopes"
  end

  test "authorize_url without drive omits the Drive scope" do
    url = Google::Client.authorize_url(redirect_uri: "http://test.host/google/callback", state: "signed-state")
    query = Rack::Utils.parse_query(URI(url).query)

    assert_equal "openid email https://www.googleapis.com/auth/calendar.events", query["scope"]
    assert_not_includes query.keys, "include_granted_scopes"
  end

  test "drive_file fetches metadata with the Drive fields" do
    stub = stub_google_drive_file("1AbcDefGhIjKlMnOpQrSt")

    file = @client.drive_file("1AbcDefGhIjKlMnOpQrSt")

    assert_equal "Q3 Planning", file["name"]
    assert_requested stub, headers: { "Authorization" => "Bearer #{@account.access_token}" }
    assert_requested :get, "#{GOOGLE_DRIVE_FILES_URL}/1AbcDefGhIjKlMnOpQrSt",
      query: hash_including({
        "fields" => "id,name,mimeType,modifiedTime,owners(displayName),webViewLink,iconLink",
        "supportsAllDrives" => "true"
      })
  end

  test "drive_file refreshes an expired access token first" do
    @account.update!(access_token_expires_at: 1.hour.ago)
    stub_google_token_refresh
    file_stub = stub_google_drive_file("1AbcDefGhIjKlMnOpQrSt")

    @client.drive_file("1AbcDefGhIjKlMnOpQrSt")

    assert_requested :post, GOOGLE_TOKEN_URL
    assert_requested file_stub, headers: { "Authorization" => "Bearer refreshed-access-token" }
  end

  test "drive_file maps 403 and 404 to NotFound" do
    stub_google_drive_file("forbidden-file-id", status: 403)
    stub_google_drive_file("missing-file-id1", status: 404)

    assert_raises(Google::Client::NotFound) { @client.drive_file("forbidden-file-id") }
    assert_raises(Google::Client::NotFound) { @client.drive_file("missing-file-id1") }
  end

  test "drive_file maps a timeout to Unavailable" do
    stub_request(:get, "#{GOOGLE_DRIVE_FILES_URL}/1AbcDefGhIjKlMnOpQrSt")
      .with(query: hash_including({ "supportsAllDrives" => "true" })).to_timeout

    error = assert_raises(Google::Client::Unavailable) { @client.drive_file("1AbcDefGhIjKlMnOpQrSt") }

    assert_equal "Google Drive request failed (Net::OpenTimeout)", error.message
  end

  test "list_drive_files requests the recent list with the Drive list parameters" do
    stub = stub_google_drive_list

    result = @client.list_drive_files(query: "")

    assert_equal [ "Q3 Planning", "Budget 2026" ], result["files"].map { |file| file["name"] }
    assert_requested stub, headers: { "Authorization" => "Bearer #{@account.access_token}" }
    assert_requested :get, GOOGLE_DRIVE_FILES_URL,
      query: {
        "q" => "trashed=false",
        "pageSize" => "10",
        "fields" => "files(id,name,mimeType,modifiedTime,owners(displayName),webViewLink)",
        "orderBy" => "modifiedTime desc",
        "spaces" => "drive"
      }
  end

  test "list_drive_files searches by name and escapes quotes and backslashes" do
    stub_google_drive_list

    @client.list_drive_files(query: "bob's\\draft")

    assert_requested :get, GOOGLE_DRIVE_FILES_URL,
      query: hash_including({ "q" => "name contains 'bob\\'s\\\\draft' and trashed=false" })
  end

  test "list_drive_files treats a blank query as a recent list" do
    stub_google_drive_list

    @client.list_drive_files(query: "   ")

    assert_requested :get, GOOGLE_DRIVE_FILES_URL,
      query: hash_including({ "q" => "trashed=false" })
  end

  test "list_drive_files refreshes an expired access token first" do
    @account.update!(access_token_expires_at: 1.hour.ago)
    stub_google_token_refresh
    list_stub = stub_google_drive_list

    @client.list_drive_files(query: "")

    assert_requested :post, GOOGLE_TOKEN_URL
    assert_requested list_stub, headers: { "Authorization" => "Bearer refreshed-access-token" }
  end

  test "429 maps to RateLimited, which retries as Unavailable" do
    stub_google_event_insert(status: 429)

    error = assert_raises(Google::Client::RateLimited) { @client.insert_event({}) }

    assert_kind_of Google::Client::Unavailable, error
    assert_equal "Google Calendar request rate limited (429)", error.message
  end

  test "a 403 carrying a quota reason maps to RateLimited" do
    stub_google_event_insert(status: 403, body: google_forbidden_body)

    error = assert_raises(Google::Client::RateLimited) { @client.insert_event({}) }

    assert_equal "Google Calendar request rate limited (rateLimitExceeded)", error.message
  end

  test "a 403 with a permission reason maps to Forbidden" do
    stub_google_event_insert(status: 403, body: google_forbidden_body("forbidden"))

    error = assert_raises(Google::Client::Forbidden) { @client.insert_event({}) }

    assert_equal "Google Calendar request forbidden (403)", error.message
  end

  test "a 403 with an unparseable body maps to Forbidden" do
    stub_request(:post, GOOGLE_EVENTS_URL).to_return(status: 403, body: "{oops")

    assert_raises(Google::Client::Forbidden) { @client.insert_event({}) }
  end

  test "410 maps to NotFound" do
    stub_google_event_delete("gone-id", status: 410)

    assert_raises(Google::Client::NotFound) { @client.delete_event("gone-id") }
  end

  test "connection failures map to Unavailable" do
    stub_request(:post, GOOGLE_EVENTS_URL).to_raise(Errno::ECONNREFUSED)

    error = assert_raises(Google::Client::Unavailable) { @client.insert_event({}) }

    assert_equal "Google Calendar request failed (Errno::ECONNREFUSED)", error.message
  end

  test "a dropped connection maps to Unavailable" do
    stub_request(:post, GOOGLE_EVENTS_URL).to_raise(EOFError)

    error = assert_raises(Google::Client::Unavailable) { @client.insert_event({}) }

    assert_equal "Google Calendar request failed (EOFError)", error.message
  end

  test "a 429 on refresh raises Unavailable and keeps the connection" do
    @account.update!(access_token_expires_at: 1.hour.ago)
    stub_request(:post, GOOGLE_TOKEN_URL).to_return(status: 429)

    assert_raises(Google::Client::Unavailable) { @client.insert_event({}) }
    assert_nil @account.reload.disconnected_reason
  end

  test "a Drive 429 maps to RateLimited" do
    stub_google_drive_file("1AbcDefGhIjKlMnOpQrSt", status: 429)

    error = assert_raises(Google::Client::RateLimited) { @client.drive_file("1AbcDefGhIjKlMnOpQrSt") }

    assert_equal "Google Drive request rate limited (429)", error.message
  end

  test "revoke_token posts the token and accepts success or already-revoked" do
    revoke = stub_google_revoke

    assert Google::Client.revoke_token("revocable-token")
    assert_requested revoke, body: hash_including({ "token" => "revocable-token" })

    WebMock.reset!
    stub_google_revoke(status: 400)

    assert Google::Client.revoke_token("revocable-token")
  end

  test "revoke_token raises Unavailable on a 5xx so the caller retries" do
    stub_google_revoke(status: 500)

    error = assert_raises(Google::Client::Unavailable) { Google::Client.revoke_token("revocable-token") }

    assert_equal "Google token revoke failed (500)", error.message
    assert_not_includes error.message, "revocable-token"
  end

  test "revoke_token raises Unavailable on a 429 so the caller retries" do
    stub_google_revoke(status: 429)

    assert_raises(Google::Client::Unavailable) { Google::Client.revoke_token("revocable-token") }
  end

  test "revoke_token returns false on other client errors" do
    stub_google_revoke(status: 403)

    assert_not Google::Client.revoke_token("revocable-token")
  end

  test "revoke_token maps transport failures to Unavailable without the token" do
    stub_request(:post, GOOGLE_REVOKE_URL).to_timeout

    error = assert_raises(Google::Client::Unavailable) { Google::Client.revoke_token("revocable-token") }

    assert_equal "Google token revoke failed (Net::OpenTimeout)", error.message
  end

  test "an unreadable token marks the account disconnected and raises Unauthorized" do
    corrupt_google_token!(@account, :access_token)

    error = assert_raises(Google::Client::Unauthorized) { @client.insert_event({}) }

    assert_equal "Google token could not be read", error.message
    assert_equal GoogleAccount::UNREADABLE_TOKEN_REASON, @account.reload.disconnected_reason
    assert_not_requested :post, GOOGLE_EVENTS_URL
  end

  test "list_events queries a single-event window with the free/busy fields mask" do
    stub_request(:get, GOOGLE_EVENTS_URL)
      .with(query: hash_including({ "singleEvents" => "true" }))
      .to_return(status: 200, body: { "items" => [] }.to_json)

    response = @client.list_events(
      time_min: Time.zone.parse("2026-09-23T09:30:00Z"),
      time_max: Time.zone.parse("2026-09-24T10:30:00Z"))

    assert_equal [], response["items"]
    assert_requested :get, GOOGLE_EVENTS_URL, query: hash_including({
      "singleEvents" => "true",
      "orderBy" => "startTime",
      "timeMin" => "2026-09-23T09:30:00Z",
      "timeMax" => "2026-09-24T10:30:00Z",
      "fields" => Google::Client::MEETING_STATUS_FIELDS
    })
  end

  test "a 429 on list_events raises RateLimited" do
    stub_request(:get, %r{\A#{GOOGLE_EVENTS_URL}}).to_return(status: 429, body: {}.to_json)

    assert_raises(Google::Client::RateLimited) do
      @client.list_events(time_min: 1.hour.ago, time_max: 1.hour.from_now)
    end
  end

  test "a quota 403 on list_events raises RateLimited" do
    stub_request(:get, %r{\A#{GOOGLE_EVENTS_URL}})
      .to_return(status: 403, body: google_forbidden_body("quotaExceeded").to_json)

    assert_raises(Google::Client::RateLimited) do
      @client.list_events(time_min: 1.hour.ago, time_max: 1.hour.from_now)
    end
  end

  test "a revoked grant on list_events disconnects and raises Unauthorized" do
    @account.update!(access_token_expires_at: 1.hour.ago)
    stub_google_token_invalid_grant

    assert_raises(Google::Client::Unauthorized) do
      @client.list_events(time_min: 1.hour.ago, time_max: 1.hour.from_now)
    end

    assert_equal "Google rejected the connection", @account.reload.disconnected_reason
  end
end
