# Execute the pinned source's actual assertion calls against the live Rust router.
# Rails supplies fixture models, encrypted test credentials and WebMock; browser
# requests/forms/Turbo/Cable and registered jobs are produced by Rust.
require "/rails/config/environment"
require "minitest/autorun"
require "active_support/test_case"
require "active_support/testing/time_helpers"
require "active_support/testing/assertions"
require "capybara"
require "capybara/dsl"
require "capybara/minitest"
require "selenium-webdriver"
require "webmock"
require "socket"
require "net/http"
require "json"
require "digest"
require "/tools/original_browser/test/test_helpers/system_test_helper"
require "/tools/original_browser/test/test_helpers/google_calendar_test_helper"
require "/tools/original_browser/test/test_helpers/web_mock_system_test_helper"

raise "wrong Capybara version" unless Capybara::VERSION == "3.40.0"
raise "wrong Selenium version" unless Selenium::WebDriver::VERSION == "4.35.0"
manifest = JSON.parse(File.read("/tools/original_browser/manifest.json"))
record = manifest.fetch("records").find { |row| row.fetch("id") == ENV.fetch("WS14_BROWSER_CASE") }
raise "missing declaration" unless record
manifest.fetch("files").each do |original, source|
  actual = "/tools/original_browser/#{original}"
  raise "pinned source changed: #{original}" unless Digest::SHA256.file(actual).hexdigest == source.fetch("sha256")
end
ActiveRecord::Base.establish_connection(adapter: "sqlite3", database: ENV.fetch("WS14_BROWSER_DATABASE"), timeout: 5000)
ActiveJob::Base.queue_adapter = :test
WebMock.enable!
WebMock.disable_net_connect!(allow_localhost: true)

# A byte-level exchange to the same WebMock stubs used by the original body.
# Only this injected external-service boundary is replaced. All same-origin
# browser traffic continues to the real Rust application.
google = TCPServer.new("127.0.0.1", 0)
File.write(File.join(ENV.fetch("WS14_BROWSER_CONTROL"), "google-port"), google.addr[1].to_s)
google_thread = Thread.new do
  loop do
    connection = google.accept
    Thread.new(connection) do |socket|
      begin
        socket.gets
        headers = {}
        while (line = socket.gets) && line != "\r\n"
          name, value = line.split(":", 2)
          headers[name.downcase] = value.strip
        end
        input = JSON.parse(socket.read(headers.fetch("content-length").to_i))
        host = input.fetch("host")
        raise "unregistered external host #{host}" unless %w[www.googleapis.com oauth2.googleapis.com api.github.com].include?(host)
        method = input.fetch("method")
        if host == "api.github.com"
          puts "WS14_GITHUB_LOCAL_REQUEST #{JSON.generate(host:, method:, target: input.fetch("target"), body: input.fetch("body"), header_names: input.fetch("headers").map(&:first), authorization_sha256: Digest::SHA256.hexdigest(input.fetch("headers").find { |name, _| name.casecmp?("Authorization") }&.last.to_s))}"
        end
        request = Net::HTTPGenericRequest.new(method, !input.fetch("body").empty?, true, input.fetch("target"), input.fetch("headers").to_h)
        request.body = input.fetch("body") unless input.fetch("body").empty?
        response = Net::HTTP.start(host, 443, use_ssl: true) { |http| http.request(request) }
        puts "WS14_GITHUB_LOCAL_RESPONSE #{response.code} #{response.body}" if host == "api.github.com"
        output = JSON.generate(status: response.code.to_i, body: response.body || "")
        socket.write "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: #{output.bytesize}\r\nConnection: close\r\n\r\n#{output}"
      rescue Exception => error
        warn "WS14_GOOGLE_BOUNDARY #{error.class}: #{error.message}"
        output = JSON.generate(status: 599, body: "unregistered Google exchange")
        socket.write "HTTP/1.1 200 OK\r\nContent-Length: #{output.bytesize}\r\nConnection: close\r\n\r\n#{output}" rescue nil
      ensure
        socket.close
      end
    end
  end
rescue IOError
  # The declaration's teardown owns this listener.
end

Capybara.enable_aria_label = true
Capybara.run_server = false
Capybara.default_max_wait_time = 2
Capybara.app_host = ENV.fetch("WS14_BROWSER_BASE")
Capybara.register_driver :ws14_original do |app|
  options = Selenium::WebDriver::Chrome::Options.new
  options.binary = "/usr/lib/chromium/chromium"
  %w[--headless=new --ozone-platform=headless --no-sandbox --disable-dev-shm-usage --mute-audio --window-size=1400,1400].each { |arg| options.add_argument(arg) }
  if ENV["WS14_BROWSER_PROXY"]
    options.add_argument "--proxy-server=#{ENV.fetch("WS14_BROWSER_PROXY")}" 
    options.add_argument "--proxy-bypass-list=<-loopback>"
  end
  Capybara::Selenium::Driver.new(app, browser: :remote, url: "http://127.0.0.1:#{ENV.fetch("WS14_BROWSER_DRIVER_PORT")}", options:)
end
Capybara.default_driver = :ws14_original

class ApplicationSystemTestCase < ActiveSupport::TestCase
  include Capybara::DSL
  include Capybara::Minitest::Assertions
  include ActiveSupport::Testing::TimeHelpers
  include ActiveSupport::Testing::Assertions
  include SystemTestHelper
  include WebMock::API
  include Rails.application.routes.url_helpers
  include ActionView::RecordIdentifier
  BROADCAST_WAIT = 15

  # These are the exact fixture IDs from the reference-built parity seed.
  def users(name)
    User.find({david: 127326141, jason: 149087659, jz: 773523953, kevin: 712064548}.fetch(name))
  end
  def rooms(name)
    Room.find({designers: 654632876, hq: 201306877, david_and_jason: 186869642}.fetch(name))
  end
  def default_url_options
    url = URI(Capybara.app_host)
    {host: url.host, port: url.port, protocol: url.scheme}
  end

  def rust_control(action, payload = {})
    response = Net::HTTP.post(URI(Capybara.app_host + "/__ws14_browser__/#{action}"), JSON.generate(payload), "Content-Type" => "application/json")
    raise "Rust #{action} failed: #{response.code} #{response.body}" unless response.code == "200"
    JSON.parse(response.body)
  end

  def perform_enqueued_jobs(only:)
    raise "unregistered original job" unless only == Calendar::MeetingRefreshJob
    result = rust_control("meeting-jobs")
    raise "original MeetingRefreshJob did not execute" unless result.fetch("performed") > 0
  end

  def travel_to(date_or_time, with_usec: false, &block)
    if block
      super(date_or_time, with_usec:) do
        rust_control("clock", time: Time.current.utc.iso8601)
        block.call
      end
      rust_control("clock", time: Time.current.utc.iso8601)
    else
      super(date_or_time, with_usec:)
      rust_control("clock", time: Time.current.utc.iso8601)
    end
  end

  setup do
    travel_to Time.utc(2026, 3, 2, 16)
    if ENV["WS14_BROWSER_PROXY"]
      visit Capybara.app_host + "/up"
      driver = page.driver.browser
      driver.extend(Selenium::WebDriver::DriverExtensions::HasCDP) unless driver.respond_to?(:execute_cdp)
      driver.execute_cdp("Network.setBypassServiceWorker", bypass: true)
    end
  end

  def before_teardown
    unless failures.empty?
      warn "WS14_ORIGINAL_DOM #{page.current_url}\n#{page.html[0, 6000]}"
      if ENV.fetch("WS14_BROWSER_CASE").start_with?("WS15g-")
        warn "WS14_ORIGINAL_GITHUB_WRITE #{page.evaluate_script("Array.from(document.querySelectorAll('.github-pr-write')).map(node => node.outerHTML)").to_json}"
        warn "WS14_ORIGINAL_GITHUB_BODY #{page.evaluate_script("document.body.innerText").to_s[-2000..]}"
      end
    end
    super
  end

  teardown do
    failures.each { |failure| warn "WS14_ORIGINAL_FAILURE #{failure.class} #{failure.message}\n#{failure.backtrace.join("\n")}" }
    Capybara.reset_sessions!
  end
end

# Keep the original dispatcher call while routing it to the real Rust API.
Calendar::MeetingDispatcher.define_singleton_method(:dispatch_due!) do
  response = Net::HTTP.post(URI(Capybara.app_host + "/__ws14_browser__/meeting-dispatch"), "{}", "Content-Type" => "application/json")
  raise "Rust meeting dispatcher failed: #{response.code}" unless response.code == "200"
end
$LOADED_FEATURES << "application_system_test_case.rb"
source = "/tools/original_browser/#{record.fetch("file")}" 
assertion_files = Hash.new { |files, file| files[file] = {} }
trace = TracePoint.new(:return) do |event|
  if event.method_id.to_s.match?(/\A(?:assert|refute)/)
    caller_locations.each do |location|
      original = location.path&.delete_prefix("/tools/original_browser/")
      assertion_files[original][location.lineno] = true if manifest.fetch("files").key?(original)
    end
  end
end
trace.enable
require source
Minitest.after_run do
  trace.disable
  files = assertion_files.transform_values { |lines| lines.keys.sort }
  lines = files.fetch(record.fetch("file"), [])
  puts "WS14_ORIGINAL_RECEIPT #{record.fetch("id")} #{JSON.generate(reference: manifest.fetch("reference"), source: record.fetch("file"), lines:, files:)}"
  google.close
  google_thread.kill
end
