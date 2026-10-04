require "json"
require "stringio"
ActionController::Base.allow_forgery_protection = false
Rails.application.config.hosts.clear
user = User.find(127326141)
Current.user = user
ActivityItem.delete_all
board = Room.find(699448332)
thread = ChannelThread.create_board_post!(room: board, creator: user, name: "Consumer <post>", work_status: "planned", first_message: "Board opener <&>")
message = thread.messages.order(:id).first!
message.update_columns(client_message_id: "ws12-consumer-opener")
# Snapshot the real owner's output before consumer requests, including inbox fanout.
tables = %w[channel_threads thread_memberships messages action_text_rich_texts work_thread_events activity_items]
conditions = { "channel_threads" => "id=#{thread.id}", "thread_memberships" => "thread_id=#{thread.id}", "messages" => "thread_id=#{thread.id}", "action_text_rich_texts" => "record_type='Message' AND record_id=#{message.id}", "work_thread_events" => "channel_thread_id=#{thread.id}", "activity_items" => "source_type='Message' AND source_id=#{message.id}" }
connection = ActiveRecord::Base.connection
inserts = ->(table, rows) { rows.map { |row| "INSERT INTO #{table} (#{row.keys.map { |key| connection.quote_column_name(key) }.join(',')}) VALUES (#{row.values.map { |value| connection.quote(value) }.join(',')})" } }
setup = ["DELETE FROM activity_items"] + tables.flat_map { |table| inserts.call(table, connection.select_all("SELECT * FROM #{table} WHERE #{conditions.fetch(table)} ORDER BY id").to_a) }
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = user.sessions.where.not(two_factor_verified_at: nil).first!.token
headers = { "Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}", "Accept" => "application/json" }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! "campfire.test"
steps = []
call = ->(label, method, path, params={}) {
 browser.public_send(method, path, params: params, headers: headers.dup, as: :json)
 body = browser.response.body
 steps << { label: label, method: method, path: path, params: params, status: browser.response.status, content_type: browser.response.headers["Content-Type"], body: body.empty? ? nil : JSON.parse(body) }
}
call.call("board save", :post, "/saved", { message_id: message.id, remind_at: "2026-03-02T16:01:00Z" })
saved = user.saved_items.find_by!(message: message)
SavedItem::ReminderDispatcher.dispatch_due!(now: Time.current + 61.seconds)
steps << { label: "dispatch reminder", operation: "remind", at: "2026-03-02T16:01:01Z" }
call.call("board reminder inbox", :get, "/activity?type=reminders")
item = saved.activity_items.first!
call.call("open reminder", :post, "/activity/#{item.id}/open")
call.call("handle reminder", :patch, "/activity/#{item.id}/handled")
call.call("re-arm reminder", :post, "/saved", { message_id: message.id, remind_at: "2026-03-02T16:02:00Z" })
SavedItem::ReminderDispatcher.dispatch_due!(now: Time.current + 121.seconds)
steps << { label: "refire reminder", operation: "remind", at: "2026-03-02T16:02:01Z" }
call.call("refired same item", :get, "/activity?type=reminders")
call.call("pin board reply", :post, "/messages/#{message.id}/pin")
call.call("schedule board reply", :post, "/rooms/#{board.id}/scheduled_messages", { scheduled_message: { markdown_source: "Scheduled **board** reply", thread_id: thread.id, reply_to_message_id: message.id, send_at: "2026-03-02T17:00:00Z" } })
scheduled = user.scheduled_messages.order(:id).last!
call.call("send board reply", :post, "/scheduled_messages/#{scheduled.id}/send_now")
call.call("board thread slash", :post, "/rooms/#{board.id}/slash_commands", { text: "/me reviews the board", thread_id: thread.id })
call.call("wrong-stream reply", :post, "/rooms/#{board.id}/scheduled_messages", { scheduled_message: { markdown_source: "Wrong stream", reply_to_message_id: message.id, send_at: "2026-03-02T17:00:00Z" } })
call.call("delete save", :delete, "/saved/#{saved.id}")
call.call("inbox after save deletion", :get, "/activity?type=reminders")
# Populated owner events, used by the production inbox JSON controller. Query count
# includes source preloading and serialization, not just the presenter itself.
pages = []
[4, 16].each do |size|
 ActivityItem.delete_all
 WorkThreadEvent.where(channel_thread_id: thread.id).delete_all
 threads = Rails.application.executor.wrap do
  Current.reset
  size.times.map do |i|
   post = ChannelThread.create_board_post!(room: board, creator: user, name: "Inbox post #{size}-#{i}", work_status: "planned")
   post.update_work!(actor: User.find(149087659), work_status: "in_progress")
   post
  end
 end
 ids = threads.map(&:id).join(',')
 rows = connection.select_all("SELECT * FROM work_thread_events WHERE channel_thread_id IN (#{ids}) ORDER BY id").to_a
 activity = connection.select_all("SELECT * FROM activity_items WHERE source_type='WorkThreadEvent' AND user_id=#{user.id} ORDER BY id").to_a
 sql = ["DELETE FROM activity_items", "DELETE FROM work_thread_events"] + inserts.call("channel_threads", connection.select_all("SELECT * FROM channel_threads WHERE id IN (#{ids}) ORDER BY id").to_a) + inserts.call("thread_memberships", connection.select_all("SELECT * FROM thread_memberships WHERE thread_id IN (#{ids}) ORDER BY id").to_a) + inserts.call("work_thread_events", rows) + inserts.call("activity_items", activity)
 queries = []
 subscriber = ->(*args) { payload=args.last; queries << payload[:sql] if !payload[:cached] && payload[:sql].match?(/\A\s*(SELECT|WITH)\b/i) && payload[:name] != "SCHEMA" }
 ActiveSupport::Notifications.subscribed(subscriber, "sql.active_record") { browser.get("/activity?type=threads", headers: headers.dup) }
 pages << { size: size, sql: sql, body: JSON.parse(browser.response.body), reads: queries.size, work_reads: queries.count { |q| q.match?(/FROM\s+"?work_thread_events"?/i) } }
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], setup: setup, thread_id: thread.id, message_id: message.id, steps: steps, pages: pages) + "\n")
puts "WS8bm2 WS12 consumer Rails: #{steps.size} workflow steps; work inbox #{pages.map { |p| "#{p[:size]} items=#{p[:reads]} reads/#{p[:work_reads]} work reads" }.join('; ')}"
