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
    { status: @client.response.status, location: @client.response.headers["Location"], json: (@client.response.parsed_body if @client.response.media_type == "application/json" && @client.response.body.present?) }.compact
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
david.room_categories.destroy_all
cases[:category_create] = admin.call(:post, "/room_categories", { room_category: { name: "Team", position: 99 } })
category = david.room_categories.reload.last
cases[:category_update] = admin.call(:patch, "/room_categories/#{category.id}", { room_category: { name: "Squad", collapsed: "true" } })
cases[:category_state] = category.reload.attributes.slice("name", "collapsed", "position")
cases[:category_list] = admin.call(:get, "/room_categories.json")
cases[:category_invalid] = admin.call(:patch, "/room_categories/#{category.id}", { room_category: {name: ""} })
cases[:category_invalid_state] = category.reload.name
cases[:category_false_name] = admin.call(:post, "/room_categories", {room_category: {name: false}})
cases[:category_false_name_state] = david.room_categories.order(:id).last.name
other = User.find(149087659).room_categories.create!(name: "Theirs")
cases[:category_cross_user] = admin.call(:patch, "/room_categories/#{other.id}", { room_category: {name: "stolen"} })
cases[:category_assignment] = admin.call(:patch, "/rooms/486777696/category_assignment.json", {room_category_id: category.id})
cases[:direct_category_assignment] = admin.call(:patch, "/rooms/186869642/category_assignment.json", {room_category_id: category.id})
cases[:inbound_rotation] = admin.call(:post, "/rooms/486777696/inbound_email_address")
cases[:direct_inbound] = admin.call(:post, "/rooms/186869642/inbound_email_address")
david.memberships.update_all(favorite_position: nil)
cases[:favorite_create] = admin.call(:post, "/rooms/486777696/favorite.json")
cases[:favorite_repeat] = admin.call(:post, "/rooms/486777696/favorite.json")
cases[:favorite_move] = admin.call(:patch, "/rooms/486777696/favorite.json", {position: "99"})
cases[:favorite_state] = david.memberships.find_by!(room_id: 486777696).favorite_position
cases[:favorite_destroy] = admin.call(:delete, "/rooms/486777696/favorite.json")
david.memberships.find_by!(room_id: 486777696).update!(unread_at: Time.current, last_read_message_id: nil)
cases[:mute_json] = admin.call(:patch, "/rooms/486777696/involvement.json", {involvement: "muted"})
muted = david.memberships.find_by!(room_id: 486777696)
cases[:mute_state] = { unread: muted.unread?, pointer: muted.last_read_message_id.present? }
cases[:missing_involvement] = admin.call(:patch, "/rooms/486777696/involvement.json")
cases[:group_rename] = member.call(:patch, "/rooms/directs/#{group.id}", {room: {name: "  Weekend  "}})
extra = User.create!(name: "Extra", email_address: "extra@example.test")
cases[:group_add] = member.call(:post, "/rooms/directs/#{group.id}/add_members", {user_ids: [extra.id]})
cases[:group_leave] = member.call(:delete, "/rooms/directs/#{group.id}/leave.json")
cases[:group_note_texts] = group.messages.where(system_note: true).order(:id).map(&:plain_text_body)
cases[:pair_add_rejected] = admin.call(:post, "/rooms/directs/186869642/add_members", {user_ids: [kevin.id]})
kevin.update_columns(status: "deactivated")
cases[:inactive_huddle_create] = admin.call(:post, "/rooms/directs", {user_ids: [149087659, kevin.id], start_huddle: "1"})
cases[:capped_before_query] = admin.call(:post, "/rooms/directs", {user_ids: [*Array.new(10, "missing"), 149087659]})
solo = Room.find(URI(cases[:capped_before_query][:location]).path.split("/").last)
cases[:solo_member_ids] = solo.user_ids
cases[:last_direct_leave] = admin.call(:delete, "/rooms/directs/#{solo.id}/leave.json")
cases[:last_direct_state] = {deleted: solo.reload.deleted?, memberships: solo.memberships.count}
ids = 10.times.map { |i| User.create!(name: "Cap #{i}", email_address: "cap#{i}@example.test").id }
cases[:over_capacity_create] = admin.call(:post, "/rooms/directs", {user_ids: ids})
plain = Current.set(user: david) { Rooms::Closed.create_for({name: "Plain"}, users: [david]) }
cases[:plain_leave] = admin.call(:delete, "/rooms/#{plain.id}/leave.json")
cases[:plain_leave_state] = {deleted: plain.reload.deleted?, memberships: plain.memberships.count}
room = Room.find(486777696)
room.update_columns(deleted_at: Time.current)
cases[:deleted_involvement] = admin.call(:get, "/rooms/#{room.id}/involvement")
room.update_columns(deleted_at: nil)
cases[:destroy_json] = admin.call(:delete, "/rooms/#{room.id}.json")
cases[:destroy_rows] = { deleted: room.reload.deleted?, memberships: room.memberships.count, audit_action: AuditLog.where(target_id: room.id).order(:id).last.action }
audit_subject = Current.set(user: david) { Rooms::Closed.create_for({name: "Audit failure"}, users: [david]) }
ActiveRecord::Base.connection.execute("CREATE TRIGGER reject_room_audit BEFORE INSERT ON audit_logs WHEN NEW.action='room.destroy' BEGIN SELECT RAISE(ABORT,'injected audit failure'); END")
cases[:audit_failure] = admin.call(:delete, "/rooms/#{audit_subject.id}.json")
cases[:audit_failure_state] = {deleted: audit_subject.reload.deleted?, enqueued: audit_subject.destroy_enqueued_at.present?}
puts JSON.pretty_generate(reference: ENV.fetch('PARITY_REFERENCE_SHA'), cases: cases)
warn "Rails room HTTP oracle: #{cases.size} cases; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
