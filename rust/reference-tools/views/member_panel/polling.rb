# Raw HTTP bytes, produced by the pinned Rails app on a fresh default seed.
require "json"
require "action_dispatch/testing/integration"
ActiveRecord::Base.logger = nil
root = ENV.fetch("PARITY_WORK")
corpus = JSON.parse(File.read("#{root}/reference-tools/views/member_panel/polling_cases.json"))
corpus["reference"] = ENV.fetch("PARITY_REFERENCE_SHA")
labels = JSON.parse(File.read("#{root}/parity/.seed/default/labels.json"))
corpus.fetch("cases").each do |entry|
  ActiveRecord::Base.transaction do
    ((entry["setup"] == false ? [] : corpus.fetch("setup")) + entry.fetch("sql")).each { |sql| ActiveRecord::Base.connection.execute(sql) }
    browser = ActionDispatch::Integration::Session.new(Rails.application)
    browser.host! "campfire.test"
    headers = { "HTTP_USER_AGENT" => "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36" }.merge(entry.fetch("headers"))
    headers["Authorization"] = "Bearer #{entry.fetch('token')}" if entry["token"]
    if entry["viewer"]
      headers["Cookie"] = "session_token=#{labels.fetch("session_cookies.#{entry.fetch('viewer')}")}"
    end
    browser.get(entry.fetch("path"), headers: headers)
    entry["status"] = browser.response.status
    entry["body"] = browser.response.body
    entry["response_headers"] = %w[ content-type cache-control pragma etag last-modified ].to_h { |name| [name, browser.response.headers[name]] }
    raise ActiveRecord::Rollback
  end
  # Nested production requests complete the runner execution context.
  ActiveSupport::ExecutionContext.clear
end
puts JSON.pretty_generate(corpus)
warn "member polling Rails fixtures: #{corpus.fetch('cases').size} raw HTTP responses"
