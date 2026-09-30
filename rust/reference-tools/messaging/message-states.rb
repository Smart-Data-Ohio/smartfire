# Real records through both the collection cache and ActionCable's actual rendered publisher.
require "json"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
Rails.cache = ActiveSupport::Cache::MemoryStore.new
room = Room.find_by!(name: "All Talk")
user = User.find_by!(email_address: "david@37signals.com")
bot = User.find(394959859)
bot.update_columns(icon_name: "github")
agent = Agent.find_by!(user: bot)
frames = []
ActionCable.server.singleton_class.prepend(Module.new do
  define_method(:broadcast) do |stream, payload, **options|
    frames << { stream:, payload: }
    super(stream, payload, **options)
  end
end)
source = room.root_messages.create!(creator: user, markdown_source: "A long reply source: " + "🎈 word " * 40, client_message_id: "states-source")
inputs = [
  ["legacy", { body: "<div><strong>Legacy</strong> &amp; text</div>" }],
  ["action", { markdown_source: "waves <hello>", action: true }],
  ["emoji", { markdown_source: "😀👍" }],
  ["streaming", { markdown_source: "Still **working**", streaming: true }],
  ["bot_icon", { markdown_source: "Bot", creator_id: bot.id }],
  ["system_note", { markdown_source: "closed a thread", system_note: true }],
  ["forward_note", { body: "<h2>Snapshot</h2><table><tbody><tr><td>kept</td></tr></tbody></table>", forwarded_markdown: true,
    forwarded_at: Time.current, forwarded_from_message_id: source.id, forward_note: "@[David]\n<note> & safe" }],
  ["legacy_forward", { body: "<h2>Snapshot</h2><table><tbody><tr><td>kept</td></tr></tbody></table>", forwarded_markdown: false, forwarded_at: Time.current, forwarded_from_message_id: source.id, forward_note: "Inherited\n<&>" }],
  ["reply", { markdown_source: "Reply", reply_to_message_id: source.id }],
  ["deleted_reply", { markdown_source: "Deleted reply", reply_to_message_id: source.id }],
  ["steps", { markdown_source: "Agent steps", creator_id: bot.id }],
  ["drive_only", { drive_file_ids: ["abcdefghij"] }],
  ["image_square", { attachment_blob_id: 1 }],
  ["image_wide", { attachment_blob_id: 7 }],
  ["video", { attachment_blob_id: 9 }],
  ["file", { attachment_blob_id: 13 }],
  ["unrepresentable_image", { attachment_blob_id: 14 }],
  ["sound_text", { markdown_source: "/play bell" }],
  ["sound_image", { markdown_source: "/play 56k" }]
]
steps = [
  { name: "<pending>", status: "pending", duration_ms: 0, input_summary: " ", output_summary: nil },
  { name: "Running", status: "running", duration_ms: 999, input_summary: "<input> &\nmore", output_summary: "" },
  { name: "Done", status: "done", duration_ms: 1250, input_summary: nil, output_summary: "done & safe" },
  { name: "Failed", status: "failed", duration_ms: nil, input_summary: "", output_summary: "<failure>" }
]
renderer = ApplicationController.renderer.new(http_host: "campfire.test", https: false)
rows = inputs.map do |name, attributes|
  message = room.root_messages.build(attributes.except(:drive_file_ids, :attachment_blob_id).merge(creator_id: attributes.fetch(:creator_id, user.id), client_message_id: "states-#{name}"))
  message.attachment = ActiveStorage::Blob.find(attributes[:attachment_blob_id]) if attributes[:attachment_blob_id]
  Array(attributes[:drive_file_ids]).each { |file_id| message.drive_attachments.build(file_id:) }
  message.save!
  source.destroy! if name == "deleted_reply"
  steps.each { |step| AgentStep.create!(step.merge(agent:, message:)) } if name == "steps"
  message.reload
  Message.preload_rendering_details([message])
  html = 2.times.map { renderer.render(partial: "messages/message", collection: [message], cached: ->(record) { ApplicationController.helpers.message_with_pr_cards_cache_key(record) }) }
  raise "cache hit changed bytes" unless html.uniq.one?
  frames.clear
  Current.reset
  message.broadcast_stream_start
  message.broadcast_stream_final
  message.broadcast_remove
  raise "publisher omitted a frame" unless frames.size == 3
  raise "session value" if ([html.first] + frames.map { |frame| frame[:payload] }).any? { |body| body.include?("authenticity_token") || body.match?(/nonce="[^"]+/) }
  { name:, input: attributes, id: message.id, html: html.first, frames: frames.dup }
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: "d7c7de92", source_id: source.id, agent_id: agent.id, steps:, rows:) + "\n")
puts "WS8bm message-states oracle: #{rows.size} real Rails states rendered cold/warm; #{rows.sum { |row| row[:frames].size }} actual append/replace/remove frames; 0 session-bound values"
