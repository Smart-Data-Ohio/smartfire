# Merge compatibility oracle. Calls our pinned clients with a fake Net::HTTP.
require "json"
require "digest"
require "net/http"
JSON.parse(File.read(ENV.fetch("GITHUB_REFERENCE_HASHES"))).each do |path, hash|
  raise "Reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
ENV["GITHUB_APP_CLIENT_ID"] = "fixture-client"
ENV["GITHUB_APP_CLIENT_SECRET"] = "fixture-secret"
module FakeTransport
  def start(host, *args, **kwargs)
    raise "Unexpected host #{host}" unless %w[github.com api.github.com].include?(host)
    Thread.current[:write_timeout] = kwargs.fetch(:write_timeout, Net::HTTP.new(host).write_timeout)
    raise Thread.current[:failure] if Thread.current[:failure]
    response = Net::HTTPOK.new("1.1", "200", "Fixture")
    response.define_singleton_method(:body) { '{"access_token":"fixture-access"}' }
    http = Object.new
    http.define_singleton_method(:post) { |*| response }
    http.define_singleton_method(:get) { |*| response }
    http.define_singleton_method(:request) { |*| response }
    yield http
  end
end
Net::HTTP.singleton_class.prepend(FakeTransport)
pr = Struct.new(:owner, :repo, :number).new("rails", "rails", 12)
calls = {
  oauth: -> { Github::App.exchange_code(code: "fixture-code", redirect_uri: "https://example.test/callback") },
  revoke: -> { Github::App.revoke_token("fixture-access") },
  write: -> { Github::WriteClient.new(token: "fixture-member").get_user },
  read: -> { Github::PullRequestFetcher.new(pr, token: nil).send(:get, "pulls/12") }
}
timeouts = calls.map do |name, call|
  call.call
  { name:, write_timeout: Thread.current[:write_timeout] }
end
errors = [Net::WriteTimeout, EOFError].flat_map do |failure|
  %i[oauth write].map do |name|
    Thread.current[:failure] = failure.new
    begin
      calls.fetch(name).call
      raise "Expected client to map transport failure"
    rescue Github::App::Error, Github::WriteClient::Error => error
      { name:, class: failure.name, message: error.message }
    ensure
      Thread.current[:failure] = nil
    end
  end
end
File.write(ENV.fetch("GITHUB_TRANSPORT_VECTOR_PATH"), JSON.pretty_generate({ reference_pin: "d7c7de92", timeouts:, errors: }) + "\n")
puts "WS15e GitHub transport Rails oracle: #{timeouts.size} write timeout defaults, #{errors.size} mapped errors; reference d7c7de92"
