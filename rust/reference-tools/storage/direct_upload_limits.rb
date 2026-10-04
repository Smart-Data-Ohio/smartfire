# Run against the pinned Rails app through Thruster AND Puma, on an isolated default seed:
#   parity/bin/reference up --seed default --port 3196 --time 2026-03-02T16:00:00Z
#   parity/bin/reference runner --port 3196 reference-tools/storage/direct_upload_limits.rb \
#     > vectors/direct_upload_limits.json
# Set PARITY_IMAGE to the image built from parity/reference.sha, and pass
# -e PARITY_REFERENCE_SHA=<that SHA> to runner. No production data is used.
require "net/http"
require "tempfile"
require "digest"
require "base64"

ApplicationJob.queue_adapter = :test
user = User.find_by!(email_address: "david@37signals.com")
session = user.sessions.first!
session.update!(two_factor_verified_at: Time.current, last_active_at: Time.current)
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed.permanent[:session_token] = { value: session.token, httponly: true, same_site: :lax }
cookies = { "session_token" => Rack::Utils.escape(request.cookie_jar[:session_token]) }
http = Net::HTTP.new("127.0.0.1", 80, nil)
http.read_timeout = http.write_timeout = 300
send_request = lambda do |method, path, headers = {}, body = nil|
  req = Net::HTTP.const_get(method.capitalize).new(URI.join("http://campfire.test", path).request_uri)
  req["Host"] = "campfire.test"
  req["Cookie"] = cookies.map { |k, v| "#{k}=#{v}" }.join("; ")
  headers.each { |k, v| req[k] = v }
  body.respond_to?(:read) ? req.body_stream = body : req.body = body
  reply = http.request(req)
  Array(reply.get_fields("set-cookie")).each do |cookie|
    key, value = cookie.split(";", 2).first.split("=", 2)
    cookies[key] = value
  end
  reply
end
profile = send_request.call("get", "/users/me/profile")
raise "profile: #{profile.code}" unless profile.code == "200"
csrf = profile.body[/name="csrf-token" content="([^"]+)"/, 1] or raise "missing CSRF token"
headers = { "Content-Type" => "application/json", "X-CSRF-Token" => csrf }
create = lambda do |size, checksum, filename = "large.bin", content_type = "application/octet-stream"|
  send_request.call("post", "/rails/active_storage/direct_uploads", headers,
    { blob: { filename: filename, byte_size: size, checksum: checksum, content_type: content_type } }.to_json)
end
cases = []
[16 * 1024 * 1024 + 1, 100_000_000].each do |size|
  Tempfile.create("upload-oracle") do |file|
    file.binmode
    remaining = size
    while remaining > 0
      chunk = "x" * [remaining, 64 * 1024].min
      file.write(chunk)
      remaining -= chunk.bytesize
    end
    file.flush
    checksum = Base64.strict_encode64(Digest::MD5.file(file.path).digest)
    metadata = create.call(size, checksum)
    raise "metadata: #{metadata.code}" unless metadata.code == "200"
    blob = JSON.parse(metadata.body)
    file.rewind
    put = send_request.call("put", blob.fetch("direct_upload").fetch("url"),
      { "Content-Type" => "application/octet-stream", "Content-Length" => size.to_s }, file)
    path = ActiveStorage::Blob.service.send(:path_for, blob.fetch("key"))
    cases << { name: "direct_#{size}", byte_size: size, checksum: checksum,
      metadata_status: metadata.code.to_i, put_status: put.code.to_i,
      stored_bytes: File.size(path), stored_checksum: Base64.strict_encode64(Digest::MD5.file(path).digest) }
    file.rewind
    bad = create.call(size, "1B2M2Y8AsgTpgAmY7PhCfg==")
    bad_blob = JSON.parse(bad.body)
    rejected = send_request.call("put", bad_blob.fetch("direct_upload").fetch("url"),
      { "Content-Type" => "application/octet-stream", "Content-Length" => size.to_s }, file)
    cases.last[:bad_checksum_status] = rejected.code.to_i
    cases.last[:bad_checksum_file_exists] = File.exist?(ActiveStorage::Blob.service.send(:path_for, bad_blob.fetch("key")))
  end
end
large_metadata = create.call(10 * 1024 * 1024 * 1024 + 1, "1B2M2Y8AsgTpgAmY7PhCfg==")
cases << { name: "metadata_over_multipart_limit", byte_size: 10 * 1024 * 1024 * 1024 + 1, metadata_status: large_metadata.code.to_i }
[0, -1, 2**63 - 1].each do |size|
  reply = create.call(size, "1B2M2Y8AsgTpgAmY7PhCfg==")
  cases << { name: "metadata_#{size}", byte_size: size, metadata_status: reply.code.to_i }
end

# The composer uses message[attachment] multipart, not the direct-upload endpoint.
Tempfile.create("composer-video") do |file|
  file.binmode
  prefix = Rails.root.join("test/fixtures/files/alpha-centuri.mov").binread
  file.write(prefix)
  file.truncate(100_000_000)
  file.flush
  checksum = Base64.strict_encode64(Digest::MD5.file(file.path).digest)
  metadata = create.call(file.size, checksum, "large.mov", "video/quicktime")
  blob = JSON.parse(metadata.body)
  file.rewind
  put = send_request.call("put", blob.fetch("direct_upload").fetch("url"),
    { "Content-Type" => "video/quicktime", "Content-Length" => file.size.to_s }, file)
  path = ActiveStorage::Blob.service.send(:path_for, blob.fetch("key"))
  cases << { name: "direct_video", fixture: "test/fixtures/files/alpha-centuri.mov", content_type: "video/quicktime",
    byte_size: file.size, checksum: checksum, metadata_status: metadata.code.to_i, put_status: put.code.to_i,
    stored_bytes: File.size(path), stored_checksum: Base64.strict_encode64(Digest::MD5.file(path).digest) }
  boundary = "----campfire-upload-oracle"
  Tempfile.create("composer-multipart") do |multipart|
    multipart.binmode
    multipart.write("--#{boundary}\r\nContent-Disposition: form-data; name=\"message[attachment]\"; filename=\"large.mov\"\r\nContent-Type: video/quicktime\r\n\r\n")
    file.rewind
    IO.copy_stream(file, multipart)
    multipart.write("\r\n--#{boundary}--\r\n")
    multipart.flush
    multipart.rewind
    reply = send_request.call("post", "/rooms/486777696/messages", {
      "Content-Type" => "multipart/form-data; boundary=#{boundary}", "Content-Length" => multipart.size.to_s,
      "Accept" => "text/vnd.turbo-stream.html", "X-CSRF-Token" => csrf
    }, multipart)
    attachment = Message.order(:id).last.attachment.blob
    path = attachment.service.send(:path_for, attachment.key)
    cases << { name: "composer_video", fixture: "test/fixtures/files/alpha-centuri.mov", byte_size: file.size,
      status: reply.code.to_i, stored_bytes: File.size(path), stored_checksum: Base64.strict_encode64(Digest::MD5.file(path).digest) }
  end
end
puts JSON.pretty_generate(reference_sha: ENV.fetch("PARITY_REFERENCE_SHA"), transport: "HTTP/1.1 through Thruster and Puma", cases: cases)
