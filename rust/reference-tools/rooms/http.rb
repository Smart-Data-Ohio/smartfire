# Actual HTTP responses from the pinned Rails application, using a private default seed.
require "json"
require "digest"
root = ENV.fetch("PARITY_WORK")
JSON.parse(File.read(File.join(root, "reference-tools/rooms/source-hashes.json"))).each do |file, hash|
  raise "reference drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest == hash
end
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
    { status: @client.response.status, location: @client.response.headers["Location"], json: (@client.response.parsed_body if @client.response.media_type == "application/json") }.compact
  end
end
david = User.find(127326141)
kevin = User.find(712064548)
admin = RoomProbe.new(david)
member = RoomProbe.new(kevin)
group = Current.set(user: kevin) { Rooms::Direct.find_or_create_for([david, User.find(149087659), kevin]) }
cases = {}
cases[:group_delete_base] = member.call(:delete, "/rooms/#{group.id}.json")
cases[:group_delete_direct] = member.call(:delete, "/rooms/directs/#{group.id}.json")
%w[Rooms::Voice Rooms::Stage Rooms::Board].each do |type|
  room = Room.create!(type: type, creator: david, name: "private history")
  room.memberships.create!(user: david)
  %w[opens closeds].each { |namespace| cases["#{type}_#{namespace}"] = admin.call(:patch, "/rooms/#{namespace}/#{room.id}", { room: {name: "leaked"} }) }
end
room = Room.find(486777696)
room.update_columns(deleted_at: Time.current)
cases[:deleted_involvement] = admin.call(:get, "/rooms/#{room.id}/involvement")
room.update_columns(deleted_at: nil)
cases[:destroy_json] = admin.call(:delete, "/rooms/#{room.id}.json")
cases[:destroy_rows] = { deleted: room.reload.deleted?, memberships: room.memberships.count, audit_action: AuditLog.where(target_id: room.id).order(:id).last.action }
puts JSON.pretty_generate(reference: "d7c7de92", cases: cases)
warn "Rails room HTTP oracle: #{cases.size} cases; reference d7c7de92"
