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
require "socket"
require "timeout"

pin = File.read(File.join(ENV.fetch("PARITY_WORK"), "parity/reference.sha")).strip
raise "reference pin mismatch" unless ENV.fetch("PARITY_REFERENCE_SHA") == pin

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

# Net::HTTP adds Content-Length: 0 to empty PUTs. Send these exact HTTP/1.1
# headers over a socket to exercise ActionDispatch::Request#content_length.
[
  ["zero_with_length", 0, "", { "Content-Length" => "0" }],
  ["zero_without_length", 0, "", {}],
  ["nonzero_without_length", 3, "", {}],
  ["chunked_matching", 3, "xxx", { "Transfer-Encoding" => "chunked" }],
  ["chunked_zero", 0, "", { "Transfer-Encoding" => "chunked" }],
  ["content_length_mismatch", 4, "xxx", { "Content-Length" => "3" }],
  ["chunked_length_mismatch", 4, "xxx", { "Transfer-Encoding" => "chunked" }]
].each do |name, size, body, framing|
  # Even the length-mismatch controls have a valid body checksum, so their 422
  # proves length validation rather than accidentally passing on checksum failure.
  checksum = Base64.strict_encode64(Digest::MD5.digest(body))
  metadata = create.call(size, checksum)
  raise "#{name} metadata: #{metadata.code}" unless metadata.code == "200"
  blob = JSON.parse(metadata.body)
  uri = URI(blob.fetch("direct_upload").fetch("url"))
  upload_headers = { "Content-Type" => "application/octet-stream" }.merge(framing)
  status = Timeout.timeout(300) do
    Socket.tcp("127.0.0.1", 80, connect_timeout: 30) do |socket|
      fields = { "Host" => "campfire.test", "Cookie" => cookies.map { |k, v| "#{k}=#{v}" }.join("; "),
        "Connection" => "close" }.merge(upload_headers)
      socket.write("PUT #{uri.request_uri} HTTP/1.1\r\n" + fields.map { |k, v| "#{k}: #{v}\r\n" }.join + "\r\n")
      if framing.key?("Transfer-Encoding")
        socket.write("#{body.bytesize.to_s(16)}\r\n#{body}\r\n") unless body.empty?
        socket.write("0\r\n\r\n")
      else
        socket.write(body)
      end
      status_line = socket.gets or raise "#{name}: missing HTTP response"
      status = status_line.split[1].to_i
      socket.read
      status
    end
  end
  path = ActiveStorage::Blob.service.send(:path_for, blob.fetch("key"))
  row = { name: name, byte_size: size, checksum: checksum, body: body, headers: upload_headers,
    metadata_status: metadata.code.to_i, put_status: status, file_exists: File.exist?(path) }
  if File.exist?(path)
    row.merge!(stored_bytes: File.size(path), stored_checksum: Base64.strict_encode64(Digest::MD5.file(path).digest))
  end
  cases << row
end

# DiskController reads params to decode its token. Keep parser failures separate
# from signed length/checksum failures; every malformed body has a valid checksum.
multipart = "--upload-boundary\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\nx\r\n--upload-boundary--\r\n"
parser_cases = [
  ["json_malformed", "application/json", "{"],
  ["json_valid", "application/json", '{"a":[1,null,"x"]}'],
  ["json_scalar", "application/json", '"hello"'],
  ["json_empty", "application/json", ""],
  ["json_block_comment", "application/json", '/*comment*/{}'],
  ["json_line_comment", "application/json", "//comment\n{}"],
  ["json_comment_at_eof", "application/json", '{}//comment'],
  ["json_comments_between_tokens", "application/json", '{/*a*/"x"/*b*/:/*c*/[1/*d*/,2]}'],
  ["json_comment_brackets", "application/json", "/*#{'[' * 101}*/{}"],
  ["json_comment_bad_utf8", "application/json", "/*\xff*/{}".b],
  ["json_comment_string", "application/json", '"/*literal*/"'],
  ["json_unterminated_comment", "application/json", '{}/*unterminated'],
  ["json_exponent_overflow", "application/json", '1e400'],
  ["json_nan", "application/json", 'NaN'],
  ["json_trailing_comma", "application/json", '[1,]'],
  ["json_trailing", "application/json", '{}x'],
  ["json_depth_100", "application/json", "[" * 100 + "0" + "]" * 100],
  ["json_depth_101", "application/json", "[" * 101 + "0" + "]" * 101],
  ["json_bad_surrogate", "application/json", '"\ud800"'],
  ["json_low_surrogate", "application/json", '"\udc00"'],
  ["json_surrogate_pair", "application/json", '"\ud800\udc00"'],
  ["json_bad_utf8", "application/json", "\"\xff\"".b],
  ["json_alias_valid", "application/problem+json", '{}'],
  ["json_alias_text", "text/x-json", "{"],
  ["json_alias_request", "application/jsonrequest", "{"],
  ["json_alias_problem", "application/problem+json", "{"],
  ["vendor_json_unparsed", "application/vnd.example+json", "{"],
  ["xml_unparsed", "application/xml", "<"],
  ["binary_unparsed", "application/octet-stream", "{"],
  ["form_valid", "application/x-www-form-urlencoded", "a=1"],
  ["form_chunked_valid", "application/x-www-form-urlencoded", "a=1", true],
  ["form_valid_empty_checksum", "application/x-www-form-urlencoded", "a=1", false, "1B2M2Y8AsgTpgAmY7PhCfg=="],
  ["form_depth_32", "application/x-www-form-urlencoded", "a" + "[x]" * 31 + "=1"],
  ["form_depth_33", "application/x-www-form-urlencoded", "a" + "[x]" * 32 + "=1"],
  ["form_depth_100", "application/x-www-form-urlencoded", "a" + "[x]" * 99 + "=1"],
  ["form_depth_101", "application/x-www-form-urlencoded", "a" + "[x]" * 100 + "=1"],
  ["form_many_params", "application/x-www-form-urlencoded", { unit: "a=1&", repeat: 4097, suffix: "a=1" }],
  ["form_over_limit", "application/x-www-form-urlencoded", { unit: "x", repeat: 4 * 1024 * 1024 + 1, suffix: "" }],
  ["form_limit_trailing_nul", "application/x-www-form-urlencoded", { unit: "x", repeat: 4 * 1024 * 1024, suffix: "\0" }],
  ["form_limit_trailing_nul_empty_checksum", "application/x-www-form-urlencoded", { unit: "x", repeat: 4 * 1024 * 1024, suffix: "\0" }, false, "1B2M2Y8AsgTpgAmY7PhCfg=="],
  ["form_limit_two_trailing_nuls", "application/x-www-form-urlencoded", { unit: "x", repeat: 4 * 1024 * 1024, suffix: "\0\0" }],
  ["form_bad_percent", "application/x-www-form-urlencoded", "a=%zz"],
  ["form_type_conflict", "application/x-www-form-urlencoded", "a=1&a[b]=2"],
  ["form_bad_utf8", "application/x-www-form-urlencoded", "a=%FF"],
  ["form_empty", "application/x-www-form-urlencoded", ""],
  ["multipart_valid", "multipart/form-data; boundary=upload-boundary", multipart],
  ["multipart_chunked_valid", "multipart/form-data; boundary=upload-boundary", multipart, true],
  ["multipart_valid_empty_checksum", "multipart/form-data; boundary=upload-boundary", multipart, false, "1B2M2Y8AsgTpgAmY7PhCfg=="],
  ["multipart_file_limit", "multipart/form-data; boundary=upload-boundary", {
    unit: "--upload-boundary\r\nContent-Disposition: form-data; name=\"a[]\"; filename=\"x.bin\"\r\n\r\nx\r\n",
    repeat: 129, suffix: "--upload-boundary--\r\n" }],
  ["multipart_part_limit", "multipart/form-data; boundary=upload-boundary", {
    unit: "--upload-boundary\r\nContent-Disposition: form-data; name=\"a[]\"\r\n\r\nx\r\n",
    repeat: 4097, suffix: "--upload-boundary--\r\n" }],
  ["multipart_related", "multipart/related; boundary=upload-boundary", multipart],
  ["multipart_mixed", "multipart/mixed; boundary=upload-boundary", multipart],
  ["multipart_truncated", "multipart/form-data; boundary=upload-boundary", multipart.sub(/--upload-boundary--\r\n\z/, '')],
  ["multipart_empty", "multipart/form-data; boundary=upload-boundary", ""],
  ["multipart_missing_boundary", "multipart/form-data", "a=1"],
  ["multipart_missing_boundary_many_params", "multipart/form-data", { unit: "a=1&", repeat: 4097, suffix: "a=1" }],
  ["multipart_missing_boundary_over_limit", "multipart/form-data", { unit: "x", repeat: 4 * 1024 * 1024 + 1, suffix: "" }],
  ["multipart_missing_boundary_bad_form", "multipart/form-data", "a=%zz"],
  ["multipart_wrong_boundary", "multipart/form-data; boundary=wrong", multipart],
  ["multipart_no_boundary_large", "multipart/form-data; boundary=upload-boundary", { unit: "x", repeat: 32768, suffix: "" }],
  ["multipart_unterminated_large_header", "multipart/form-data; boundary=upload-boundary", {
    prefix: "--upload-boundary\r\nX-Filler: ", unit: "x", repeat: 65537, suffix: "" }],
  ["multipart_long_boundary", "multipart/form-data; boundary=#{'x' * 71}", multipart],
  ["multipart_boundary_space", "multipart/form-data; boundary =upload-boundary", multipart],
  ["multipart_boundary_duplicate", "multipart/form-data; boundary=upload-boundary; boundary=again", multipart]
]
empty_checksum = Base64.strict_encode64(Digest::MD5.digest(""))
multipart_type = "multipart/form-data; boundary=upload-boundary"
file_part = "--upload-boundary\r\nContent-Disposition: form-data; name=\"a[]\"; filename=\"x.bin\"\r\n\r\nx\r\n"
blank_part = file_part.sub('filename="x.bin"', 'filename=""')
text_part = "--upload-boundary\r\nContent-Disposition: form-data; name=\"a[]\"\r\n\r\nx\r\n"
[127, 128].each do |count|
  parser_cases << ["multipart_files_#{count}", multipart_type, { unit: file_part, repeat: count, suffix: "--upload-boundary--\r\n" }]
end
[4095, 4096].each do |count|
  parser_cases << ["multipart_parts_#{count}", multipart_type, { unit: text_part, repeat: count, suffix: "--upload-boundary--\r\n" }]
end
[127, 128, 129].each do |count|
  parser_cases << ["multipart_blank_files_#{count}", multipart_type, { unit: blank_part, repeat: count, suffix: "--upload-boundary--\r\n" }]
end
parser_cases << ["multipart_blank_files_127_empty_checksum", multipart_type,
  { unit: blank_part, repeat: 127, suffix: "--upload-boundary--\r\n" }, false, empty_checksum]
parser_cases << ["multipart_mixed_blank_file_limit", multipart_type,
  { prefix: blank_part * 64, unit: file_part, repeat: 64, suffix: "--upload-boundary--\r\n" }]
# Rack reads exactly 1 MiB chunks and leaves the suffix after that read-ahead for DiskService.
[ ["large", 2_000_000], ["at_read_boundary", 1_048_576 - multipart.bytesize],
  ["past_read_boundary", 1_048_577 - multipart.bytesize] ].each do |label, size|
  recipe = { prefix: multipart, unit: "x", repeat: size, suffix: "" }
  parser_cases << ["multipart_epilogue_#{label}_empty_checksum", multipart_type, recipe, false, empty_checksum]
  if label != "at_read_boundary"
    suffix_checksum = Base64.strict_encode64(Digest::MD5.digest("x" * (multipart.bytesize + size - 1_048_576)))
    parser_cases << ["multipart_epilogue_#{label}_suffix_checksum", multipart_type, recipe, false, suffix_checksum]
  end
end
parser_cases.each do |name, content_type, body, chunked, signed_checksum|
  recipe = body if body.is_a?(Hash)
  body = recipe.fetch(:prefix, "") + recipe[:unit] * recipe[:repeat] + recipe[:suffix] if recipe
  checksum = signed_checksum || Base64.strict_encode64(Digest::MD5.digest(body))
  metadata = create.call(body.bytesize, checksum, "parser.bin", content_type.split(';', 2).first)
  raise "#{name} metadata: #{metadata.code}" unless metadata.code == "200"
  blob = JSON.parse(metadata.body)
  framing = chunked ? { "Transfer-Encoding" => "chunked" } : { "Content-Length" => body.bytesize.to_s }
  status = if chunked
    uri = URI(blob.fetch("direct_upload").fetch("url"))
    Timeout.timeout(300) do
      Socket.tcp("127.0.0.1", 80, connect_timeout: 30) do |socket|
        fields = { "Host" => "campfire.test", "Cookie" => cookies.map { |k, v| "#{k}=#{v}" }.join("; "),
          "Connection" => "close", "Content-Type" => content_type }.merge(framing)
        socket.write("PUT #{uri.request_uri} HTTP/1.1\r\n" + fields.map { |k, v| "#{k}: #{v}\r\n" }.join + "\r\n")
        socket.write("#{body.bytesize.to_s(16)}\r\n#{body}\r\n") unless body.empty?
        socket.write("0\r\n\r\n")
        status = socket.gets.split[1].to_i
        socket.read
        status
      end
    end
  else
    send_request.call("put", blob.fetch("direct_upload").fetch("url"),
      { "Content-Type" => content_type }.merge(framing), body).code.to_i
  end
  path = ActiveStorage::Blob.service.send(:path_for, blob.fetch("key"))
  cases << { name: name, kind: "parser", byte_size: body.bytesize, checksum: checksum,
    content_type: content_type, framing: framing,
    metadata_status: metadata.code.to_i, put_status: status, file_exists: File.exist?(path) }
  cases.last.merge!(recipe ? { body_recipe: recipe } : { body_base64: Base64.strict_encode64(body) })
  if File.exist?(path)
    cases.last.merge!(stored_bytes: File.size(path), stored_checksum: Base64.strict_encode64(Digest::MD5.file(path).digest))
  end
end

# No parser runs before DiskController's session gate. Include the metadata POST
# controls too: that separate controller retains CSRF/parameter parsing before its gate.
[
  ["anonymous_json_malformed", "application/json", "{"],
  ["anonymous_json_comment", "application/json", "/*comment*/{}"],
  ["anonymous_json_alias_text", "text/x-json", "{"],
  ["anonymous_json_alias_request", "application/jsonrequest", "{"],
  ["anonymous_json_alias_problem", "application/problem+json", "{"],
  ["anonymous_form_malformed", "application/x-www-form-urlencoded", "a=%zz"],
  ["anonymous_multipart_truncated", multipart_type, multipart.sub(/--upload-boundary--\r\n\z/, '')],
  ["anonymous_multipart_limit", multipart_type, file_part * 128 + "--upload-boundary--\r\n"]
].each do |name, content_type, body|
  checksum = Base64.strict_encode64(Digest::MD5.digest(body))
  metadata = create.call(body.bytesize, checksum, "anonymous.bin", content_type.split(';', 2).first)
  raise "#{name} metadata: #{metadata.code}" unless metadata.code == "200"
  blob = JSON.parse(metadata.body)
  put = send_request.call("put", blob.fetch("direct_upload").fetch("url"),
    { "Cookie" => "", "Content-Type" => content_type, "Content-Length" => body.bytesize.to_s }, body)
  path = ActiveStorage::Blob.service.send(:path_for, blob.fetch("key"))
  cases << { name: name, kind: "authentication", byte_size: body.bytesize, checksum: checksum,
    content_type: content_type, body_base64: Base64.strict_encode64(body),
    metadata_status: metadata.code.to_i, put_status: put.code.to_i, file_exists: File.exist?(path) }
end
[ ["anonymous_metadata_malformed", "{"], ["anonymous_metadata_valid_json", "{}"] ].each do |name, body|
  count = ActiveStorage::Blob.count
  reply = send_request.call("post", "/rails/active_storage/direct_uploads",
    { "Cookie" => "", "Content-Type" => "application/json" }, body)
  cases << { name: name, kind: "metadata_authentication", body: body,
    status: reply.code.to_i, allocated_blobs: ActiveStorage::Blob.count - count }
end

body = "xxx"
checksum = Base64.strict_encode64(Digest::MD5.digest(body))
metadata = create.call(body.bytesize, checksum)
blob = JSON.parse(metadata.body)
path = ActiveStorage::Blob.service.send(:path_for, blob.fetch("key"))
[ ["complete_upload", "xxx"], ["corrupt_retry", "yyy"], ["retry_after_corruption", "xxx"] ].each do |name, bytes|
  put = send_request.call("put", blob.fetch("direct_upload").fetch("url"),
    { "Content-Type" => "application/octet-stream", "Content-Length" => bytes.bytesize.to_s }, bytes)
  cases << { name: name, kind: "retry", byte_size: bytes.bytesize, checksum: checksum, body: bytes,
    metadata_status: metadata.code.to_i, put_status: put.code.to_i, file_exists: File.exist?(path) }
  if File.exist?(path)
    cases.last.merge!(stored_bytes: File.size(path), stored_checksum: Base64.strict_encode64(Digest::MD5.file(path).digest))
  end
end

# Source/write errors are not IntegrityError. Capture DiskService's copied bytes,
# rather than interpreting its rescue-less IO.copy_stream as checksum cleanup.
Dir.mktmpdir("disk-failure-oracle") do |root|
  service = ActiveStorage::Service::DiskService.new(root: root)
  failing_reader = Class.new do
    def initialize; @read = false; end
    def read(length = nil, buffer = nil)
      raise IOError, "injected source read failure" if @read
      @read = true
      buffer ? buffer.replace("partial body") : "partial body"
    end
  end
  [false, true].each do |retry_upload|
    key = retry_upload ? "readerretry" : "readerfresh"
    previous = retry_upload ? "previous complete blob" : nil
    service.upload(key, StringIO.new(previous)) if previous
    begin
      service.upload(key, failing_reader.new, checksum: "ignored")
      raise "reader injection did not fail"
    rescue IOError => error
      path = service.send(:path_for, key)
      cases << { name: "reader_error_#{retry_upload ? 'retry' : 'fresh'}", kind: "service_failure",
        previous_body: previous, error: error.class.name,
        status: ActionDispatch::ExceptionWrapper.status_code_for_exception(error.class.name),
        file_exists: File.exist?(path), stored_body: File.binread(path) }
    end
  end
end

# Bound file writes in a child process, leaving the oracle/server limits unchanged.
[false, true].each do |retry_upload|
  reader, writer = IO.pipe
  pid = fork do
    reader.close
    Dir.mktmpdir("disk-write-oracle") do |root|
      service = ActiveStorage::Service::DiskService.new(root: root)
      key = "boundedwrite"
      previous = retry_upload ? "previous complete blob" : nil
      service.upload(key, StringIO.new(previous)) if previous
      Signal.trap("XFSZ", "IGNORE")
      Process.setrlimit(:FSIZE, 4096, 4096)
      begin
        service.upload(key, StringIO.new("x" * 65536), checksum: "ignored")
        raise "write injection did not fail"
      rescue Errno::EFBIG => error
        path = service.send(:path_for, key)
        Marshal.dump({ name: "write_error_#{retry_upload ? 'retry' : 'fresh'}", kind: "service_failure",
          previous_body: previous, error: error.class.name,
          status: ActionDispatch::ExceptionWrapper.status_code_for_exception(error.class.name),
          file_exists: File.exist?(path), stored_bytes: File.size(path),
          stored_checksum: Base64.strict_encode64(Digest::MD5.file(path).digest) }, writer)
      end
    end
    writer.close
    exit! 0
  end
  writer.close
  cases << Marshal.load(reader)
  reader.close
  Process.wait(pid)
  raise "write probe failed" unless $?.success?
end

Tempfile.create("json-upload-oracle") do |file|
  file.binmode
  value_bytes = 16 * 1024 * 1024 + 1
  file.write('"')
  remaining = value_bytes
  while remaining > 0
    chunk = "x" * [remaining, 64 * 1024].min
    file.write(chunk)
    remaining -= chunk.bytesize
  end
  file.write('"')
  file.flush
  checksum = Base64.strict_encode64(Digest::MD5.file(file.path).digest)
  metadata = create.call(file.size, checksum, "large.json", "application/json")
  blob = JSON.parse(metadata.body)
  file.rewind
  put = send_request.call("put", blob.fetch("direct_upload").fetch("url"),
    { "Content-Type" => "application/json", "Content-Length" => file.size.to_s }, file)
  path = ActiveStorage::Blob.service.send(:path_for, blob.fetch("key"))
  cases << { name: "json_over_16_mib", kind: "large_json", value_bytes: value_bytes,
    byte_size: file.size, checksum: checksum, metadata_status: metadata.code.to_i, put_status: put.code.to_i,
    file_exists: File.exist?(path), stored_bytes: File.size(path), stored_checksum: Base64.strict_encode64(Digest::MD5.file(path).digest) }
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
