# Actual pinned Rails quick-switcher payload for the private parity seed.
require "json"
require "digest"
root = ENV.fetch("PARITY_WORK")
hash = "69c1cd75c027ca0dabe81d300fceb83ac07155f7f0fcc8cbf1350ec087219433"
raise "reference drift: switchers" unless Digest::SHA256.file(Rails.root.join("app/controllers/switchers_controller.rb")).hexdigest == hash
class RoomProbe
  attr_reader :client
  def initialize(user)
    @client = ActionDispatch::Integration::Session.new(Rails.application)
    @client.host! "campfire.test"
    request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
    request.cookie_jar.signed.permanent[:session_token] = { value: user.sessions.first.token, httponly: true, same_site: :lax }
    @client.cookies["session_token"] = request.cookie_jar[:session_token]
    @client.get "/account/edit"
    ActiveSupport::IsolatedExecutionState.clear
    @token = Nokogiri::HTML(@client.response.body).at_css('meta[name="csrf-token"]')&.[]("content")
    raise "no CSRF token: #{@client.response.status}" unless @token
  end
  def call(method, path, params = {})
    @client.public_send(method, path, params: params, headers: { "X-CSRF-Token" => @token, "Accept" => "application/json" })
    ActiveSupport::IsolatedExecutionState.clear
    { status: @client.response.status, location: @client.response.headers["Location"], json: (@client.response.parsed_body if @client.response.media_type == "application/json" && @client.response.body.present?) }.compact
  end
end
david = User.find(127326141)
admin = RoomProbe.new(david)
result = {seed: admin.call(:get, "/switcher.json").fetch(:json)}
result[:seed_body] = admin.client.response.body
Room.find(486777696).update!(name: "Escaped <&>\u2028\u2029")
admin.call(:get, "/switcher.json")
result[:escaped_body] = admin.client.response.body
anonymous = ActionDispatch::Integration::Session.new(Rails.application)
anonymous.host! "campfire.test"
anonymous.get "/switcher.json"
ActiveSupport::IsolatedExecutionState.clear
result[:anonymous_status] = anonymous.response.status
anonymous.get "/switcher.json", params: {bot_key: "394959859-BenderToken1"}
ActiveSupport::IsolatedExecutionState.clear
result[:bot_status] = anonymous.response.status
warn "Rails switcher oracle: 2 byte payloads, 2 auth responses; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
puts JSON.pretty_generate(result)
