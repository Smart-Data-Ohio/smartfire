# Cache review: warm the real Rails room, change one rendered input, then reload.
require "json"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
ApplicationController.allow_forgery_protection = false
Rails.application.config.hosts.clear
Rails.cache = ActiveSupport::Cache::MemoryStore.new
ActionView::PartialRenderer.collection_cache = Rails.cache
ApplicationController.perform_caching = true
room = Room.find(486777696)
david = User.find(127326141)
jason = User.find(149087659)
original_jason = jason.attributes.slice("name", "bio", "updated_at")
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = david.sessions.where.not(two_factor_verified_at: nil).first!.token
headers = { "Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! "campfire.test"
renderer = ApplicationController.renderer.new(http_host: "campfire.test", https: false, "rack.session" => {})
models = [Message, ActionText::RichText, Boost, Poll, PollOption, PollVote, ActiveStorage::Blob, ActiveStorage::Attachment, MessageReference, WorkspaceIcon, LinkEmbed, LinkEmbedReference]
existing = models.to_h { |model| [model, model.pluck(:id)] }
make = ->(name, **attrs) { room.root_messages.create!(creator: david, markdown_source: "Dependency #{name}", client_message_id: "dependency-#{name}", **attrs) }
cases = []
%w[reactor legacy_booster mention reply_mention quote_mention poll_voter poll_option room direct_room_member attachment reply_attachment icon link_reference poll_clock].each do |name|
  message = make.call(name)
  case name
  when "reactor", "legacy_booster"
    message.boosts.create!(booster: jason, content: name == "reactor" ? "👍" : "Great work")
  when "mention"
    message.update!(markdown_source: "Hello @[#{jason.name}]")
  when "reply_mention", "quote_mention"
    source = make.call("#{name}-source", markdown_source: "Hello @[#{jason.name}]")
    if name == "reply_mention"
      message.update!(reply_to_message: source)
    else
      message.message_references.create!(referenced_message: source)
    end
  when "poll_voter", "poll_option"
    poll = Poll.create_for_message!(message:, labels: ["Before", "Other"])
    poll.cast_vote!(jason, [poll.poll_options.first.id]) if name == "poll_voter"
  when "direct_room_member"
    message.update!(room: Room.find(186869642))
  when "icon"
    icon = WorkspaceIcon.create!(creator: david, name: "review_icon", title: "Before icon", image: {
      io: StringIO.new('<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><path d="M0 0h64v64H0z"/></svg>'), filename: "review.svg", content_type: "image/svg+xml" })
    blob = icon.image.blob
    destination = blob.service.path_for("review-icon-file")
    FileUtils.mkdir_p(File.dirname(destination))
    FileUtils.mv(blob.service.path_for(blob.key), destination)
    blob.update_columns(key: "review-icon-file")
    message.boosts.create!(booster: david, content: ":review_icon:")
  when "link_reference"
    embed = LinkEmbed.create!(normalized_url: "https://example.test/review", title: "Review card", fetched_at: Time.current, expires_at: 1.day.from_now)
    message.link_embed_references.create!(link_embed: embed, url: "https://example.test/review#before")
  when "poll_clock"
    Poll.create_for_message!(message:, labels: ["Before", "Other"], closes_at: Time.current + 0.100.seconds)
  when "attachment", "reply_attachment"
    target = name == "attachment" ? message : make.call("attachment-source")
    target.attachment.attach(io: StringIO.new("review fixture"), filename: "before.txt", content_type: "text/plain")
    target.attachment.blob.update_columns(key: "review-#{name}-file")
    target.update!(markdown_source: nil, body: "")
    message.update!(reply_to_message: target) if name == "reply_attachment"
  end
  cases << { name:, message: }
end
# These setup rows are fixture data, not mutations or credentials. Keep database timestamp
# strings intact so the Rust fixture compares the exact Rails HTML and record versions.
database = models.to_h do |model|
  rows = model.where.not(id: existing.fetch(model)).order(:id).map do |record|
    model.connection.select_one("SELECT * FROM #{model.table_name} WHERE id=#{record.id}")
  end
  [model.table_name, rows]
end
rendered_keys = ->(message) do
  sources = [message, message.reply_to_message, *message.referenced_messages].compact.uniq
  rooms = [message.room, *message.referenced_messages.map(&:room)].uniq
  users = sources.map(&:creator) + message.boosts.map(&:booster)
  users += sources.flat_map { |source| source.body.body&.attachables&.grep(User) || [] }
  users += message.poll&.poll_votes&.filter_map(&:user) || []
  records = sources + users + sources.filter_map(&:rich_text_body)
  records += [*message.boosts, *message.message_pins, *message.agent_steps, *message.drive_attachments,
    message.poll, *message.poll&.poll_options, *message.poll&.poll_votes,
    *message.message_references, *message.github_pull_request_references, *message.github_pull_requests,
    *message.fizzy_card_references, *message.fizzy_cards, *message.twitter_post_references, *message.twitter_posts,
    *message.event_references, *message.events, *message.link_embed_references, *message.link_embeds]
  records += Github::PullRequestThread.where(room_id: message.room_id, github_pull_request_id: message.github_pull_requests.map(&:id)).to_a if message.github_pull_requests.any?
  attachments = sources.filter_map { |source| source.attachment_attachment } + users.uniq.filter_map(&:avatar_attachment)
  attachments += sources.filter_map(&:rich_text_body).flat_map(&:embeds_attachments)
  records += attachments + attachments.map(&:blob)
  # Project room-wide state to the labels actually rendered; posting only touches updated_at.
  room_keys = rooms.map do |room|
    label = room.id == message.room_id ? ApplicationController.helpers.room_display_name(room, for_user: nil) : ApplicationController.helpers.viewer_neutral_room_label(room)
    ActiveSupport::Cache.expand_cache_key(["#{room.model_name.cache_key}/#{room.id}", label.to_s])
  end
  icon_names = message.boosts.filter_map { |boost| boost.content.strip[/\A:([a-z0-9_]+):\z/, 1] }
  avatar_users = [message.creator, *message.boosts.map(&:booster)].uniq
  icon_names += avatar_users.select { |user| user.bot? && !user.avatar.attached? }.filter_map(&:icon_name)
  sources.each do |source|
    next unless source.markdown? || source.forwarded_markdown?
    icon_names += Nokogiri::HTML5.fragment(source.body.body.to_html).css("img[alt]").filter_map { |img| img["alt"][/\A:([a-z0-9_]+):\z/, 1] }
  end
  custom_names = icon_names.filter_map { |name| icon = Icons.find(name); icon.name if icon.is_a?(Icons::Custom) }.uniq
  records += WorkspaceIcon.where(name: custom_names).to_a
  key = (records.compact.map(&:cache_key_with_version) + room_keys).uniq.sort
  key << message.poll.closed? if message.poll
  ActiveSupport::Cache.expand_cache_key(key)
end
capture = ->(message) do
  2.times do
    browser.get("/rooms/#{message.room_id}", headers:)
    raise "room status #{browser.response.status}" unless browser.response.successful?
  end
  Rails.application.executor.wrap do
    message.reload
    Message.preload_rendering_details([message])
    html = renderer.render(partial: "messages/message", locals: { message: })
    raise "room did not mount #{message.id}" unless browser.response.body.include?(html)
    { html:, record_key: rendered_keys.call(message) }
  end
end
rows = cases.map do |entry|
  name, message = entry.values_at(:name, :message)
  before = capture.call(message)
  travel_to Time.utc(2026, 3, 2, 16) + 0.125.seconds, with_usec: true
  mutation = case name
  when "reactor", "legacy_booster", "mention", "reply_mention", "quote_mention", "poll_voter", "direct_room_member"
    jason.update!(name: "Renamed dependency user", bio: "Updated dependency bio")
    { table: "users", id: jason.id, attributes: { name: jason.name, bio: jason.bio, updated_at: jason.updated_at.iso8601(6) } }
  when "room"
    message.room.update!(name: "Renamed dependency room")
    { table: "rooms", id: message.room_id, attributes: { name: message.room.name, updated_at: message.room.updated_at.iso8601(6) } }
  when "poll_option"
    option = message.poll.poll_options.first
    option.update!(label: "After")
    { table: "poll_options", id: option.id, attributes: { label: option.label, updated_at: option.updated_at.iso8601(6) } }
  when "icon"
    icon = WorkspaceIcon.find_by!(name: "review_icon")
    icon.update!(title: "After icon")
    { table: "workspace_icons", id: icon.id, attributes: { title: icon.title, updated_at: icon.updated_at.iso8601(6) } }
  when "link_reference"
    reference = message.link_embed_references.first
    reference.update!(url: "https://example.test/review#after")
    { table: "link_embed_references", id: reference.id, attributes: { url: reference.url, updated_at: reference.updated_at.iso8601(6) } }
  when "poll_clock"
    { table: "clock", id: 0, attributes: {} }
  when "attachment", "reply_attachment"
    target = name == "attachment" ? message : message.reply_to_message
    target.attachment.blob.update!(filename: "after.txt")
    { table: "active_storage_blobs", id: target.attachment.blob.id, attributes: { filename: "after.txt" }, touched_message_id: target.id }
  end
  after = capture.call(message)
  raise "unchanged #{name}" if before[:html] == after[:html]
  # Restore before the next independent case, including room touches from blobs.
  result = { name:, message_id: message.id, room_id: message.room_id, mutation:, before: before[:html], after: after[:html], before_key: before[:record_key], after_key: after[:record_key] }
  model = models.find { |candidate| candidate.table_name == mutation[:table] }
  if model && (original = database.fetch(model.table_name).find { |record| record["id"] == mutation[:id] })
    model.find(mutation[:id]).update_columns(original.except("id"))
  end
  if mutation[:touched_message_id]
    original = database.fetch("messages").find { |record| record["id"] == mutation[:touched_message_id] }
    Message.find(original.fetch("id")).update_columns(updated_at: original.fetch("updated_at"))
    room.update_columns(updated_at: Time.utc(2026, 3, 2, 16))
  end
  Icons.expire_custom_cache! if name == "icon"
  jason.update_columns(original_jason) if mutation[:table] == "users"
  room.update_columns(name: "All Talk", updated_at: Time.utc(2026, 3, 2, 16)) if name == "room"
  travel_to Time.utc(2026, 3, 2, 16)
  Rails.cache.clear
  result
end
controller = ApplicationController.new
controller.set_request!(ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new)))
controller.set_response!(ActionDispatch::Response.new)
view = controller.view_context
name = "messages/_message"
digest = ActionView::Digestor.digest(name:, format: :html, finder: view.lookup_context)
tree = ActionView::Digestor.tree(name, view.lookup_context, true)
walk = ->(node) { [node.name, *node.children.flat_map { |child| walk.call(child) }] }
template_dependencies = walk.call(tree).uniq.sort
File.write(File.join(File.dirname(ARGV.fetch(0)), "message-template-digest.txt"), digest + "\n")
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: "d7c7de92", template_digest: digest, template_dependencies:, database:, rows:) + "\n")
puts "WS8bm rendered dependencies: #{rows.size} warm-room update/reload pairs from pinned Rails"
