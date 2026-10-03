# Real Rack entrypoint and HTTP/1.1, including Rack::Deflater.
require "active_support/testing/time_helpers"
require "base64"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
Rails.logger = ActiveSupport::Logger.new($stderr)
app = Rack::Builder.parse_file(Rails.root.join("config.ru").to_s)
Rails.application.config.content_security_policy_nonce_generator = ->(_request) { "AAECAwQFBgcICQoLDA0ODw==" }
def all_headers(reply)
  reply.headers.each_with_object({}) do |(header, value), result|
    name = header.downcase
    raise "duplicate header name #{name}" if result.key?(name)
    values = Array(value)
    raise "duplicate header values #{name}: #{values.inspect}" unless values.length == 1
    result[name] = values
  end
end
travel_to Time.utc(2026, 3, 2, 16) do
  blob = ActiveStorage::Blob.create_and_upload!(key: "ws11api-next2-blob", io: StringIO.new("proxy header fixture\n"), filename: "fixture.txt", content_type: "text/plain")
  File.utime(Time.current.to_time, Time.current.to_time, blob.service.send(:path_for, blob.key))
  path = Rails.application.routes.url_helpers.rails_storage_proxy_path(blob, only_path: true)
  session = ActionDispatch::Integration::Session.new(app)
  session.host! "campfire.test"
  before = %w[active_storage_blobs active_storage_attachments active_storage_variant_records].to_h { |table| [table, ActiveRecord::Base.connection.select_all("SELECT * FROM #{table} ORDER BY id").to_a] }
  ApplicationJob.queue_adapter.enqueued_jobs.clear
  cases = [["whole", nil], ["range", "bytes=0-4"], ["unsatisfiable", "bytes=9999-"], ["missing", nil]].map do |name, range|
    blob.service.delete(blob.key) if name == "missing"
    headers = { "Accept" => "*/*", "SERVER_PROTOCOL" => "HTTP/1.1" }
    headers["Range"] = range if range
    session.get(path, headers: headers)
    raise "wrong protocol" unless session.request.env.fetch("SERVER_PROTOCOL") == "HTTP/1.1"
    reply = session.response
    { request_protocol: session.request.env.fetch("SERVER_PROTOCOL"), name: name, range: range, status: reply.status, headers: all_headers(reply), body_base64: Base64.strict_encode64(reply.body.b), body_bytes: reply.body.bytesize }
  end
  after = before.keys.to_h { |table| [table, ActiveRecord::Base.connection.select_all("SELECT * FROM #{table} ORDER BY id").to_a] }
  raise "blob proxy mutated rows or jobs" unless before == after && ApplicationJob.queue_adapter.enqueued_jobs.empty?
  puts JSON.pretty_generate({ reference_pin: "d7c7de92", notes: ["config.ru Rack::Deflater, HTTP/1.1, no Accept-Encoding. Fixed key, mtime and nonce entropy are fixture inputs. Every header and body byte retained. Six exact Rust security additions and only Date/X-Request-Id/X-Runtime values are approved differences."], blob_id: blob.id, path: path, rows_unchanged: true, jobs: [], cases: cases })
end
