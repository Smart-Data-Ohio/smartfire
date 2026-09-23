require "test_helper"

class ActiveStorageAuthenticationTest < ActionDispatch::IntegrationTest
  setup do
    host! "smartfire.test"
  end

  test "direct upload metadata endpoint rejects anonymous callers" do
    get new_session_url
    assert_response :success

    assert_no_difference -> { ActiveStorage::Blob.count } do
      post rails_direct_uploads_url, params: blob_params, as: :json
    end

    assert_response :unauthorized
  end

  test "direct upload metadata endpoint allows authenticated users" do
    sign_in :david

    assert_difference -> { ActiveStorage::Blob.count }, 1 do
      post rails_direct_uploads_url, params: blob_params, as: :json
    end

    assert_response :success
  end

  test "direct upload metadata endpoint allows two-factor-verified sessions" do
    credential = enroll_two_factor!(users(:david))
    post session_url, params: { email_address: users(:david).email_address, password: "secret123456" }
    post two_factor_challenge_url, params: { code: totp_code_for(credential) }
    assert_redirected_to root_url

    assert_difference -> { ActiveStorage::Blob.count }, 1 do
      post rails_direct_uploads_url, params: blob_params, as: :json
    end

    assert_response :success
  end

  test "direct upload metadata endpoint rejects unenrolled sessions" do
    post session_url, params: { email_address: users(:david).email_address, password: "secret123456" }
    assert cookies[:session_token].present?

    assert_no_difference -> { ActiveStorage::Blob.count } do
      post rails_direct_uploads_url, params: blob_params, as: :json
    end

    assert_response :unauthorized
  end

  test "direct upload metadata endpoint rejects stale enrolled sessions" do
    post session_url, params: { email_address: users(:david).email_address, password: "secret123456" }
    enroll_two_factor!(users(:david))

    assert_no_difference -> { ActiveStorage::Blob.count } do
      post rails_direct_uploads_url, params: blob_params, as: :json
    end

    assert_response :unauthorized
  end

  test "disk service upload endpoint rejects unverified sessions" do
    sign_in :david
    post rails_direct_uploads_url, params: blob_params, as: :json
    assert_response :success
    upload_path = URI.parse(response.parsed_body.dig("direct_upload", "url")).request_uri
    blob = ActiveStorage::Blob.find_signed!(response.parsed_body["signed_id"])

    unverified = open_session
    unverified.host! "smartfire.test"
    unverified.post session_path, params: { email_address: users(:jason).email_address, password: "secret123456" }
    unverified.put upload_path,
      params: attachment_bytes,
      headers: { "Content-Type" => "application/octet-stream" }

    assert_equal 401, unverified.status

    put upload_path,
      params: attachment_bytes,
      headers: { "Content-Type" => "application/octet-stream" }

    assert_response :no_content
  ensure
    blob&.purge
  end

  test "disk service upload endpoint rejects anonymous callers" do
    sign_in :david
    post rails_direct_uploads_url, params: blob_params, as: :json
    assert_response :success
    upload_path = URI.parse(response.parsed_body.dig("direct_upload", "url")).request_uri

    anonymous = open_session
    anonymous.host! "smartfire.test"
    anonymous.put upload_path,
      params: attachment_bytes,
      headers: { "Content-Type" => "application/octet-stream" }

    assert_equal 401, anonymous.status
  end

  test "disk service download endpoint stays public" do
    ActiveStorage::Current.url_options = { host: "smartfire.test", protocol: "https" }
    blob = ActiveStorage::Blob.create_and_upload! \
      io: StringIO.new(attachment_bytes), filename: "hi.txt", content_type: "text/plain"
    download_path = URI.parse(blob.url).request_uri

    anonymous = open_session
    anonymous.host! "smartfire.test"
    anonymous.get download_path

    assert_equal 200, anonymous.status
    assert_equal attachment_bytes, anonymous.response.body
  ensure
    blob&.purge
  end

  private
    def attachment_bytes
      "hello!"
    end

    def blob_params
      { blob: {
        filename: "quota.bin",
        byte_size: attachment_bytes.bytesize,
        checksum: Digest::MD5.base64digest(attachment_bytes),
        content_type: "application/octet-stream"
      } }
    end
end
