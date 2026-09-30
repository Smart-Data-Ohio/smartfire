# Whole messages from the default parity seed, plus actual attachment-presentation output for
# ActiveStorage::Filename#base edge cases. Run images.sh after run.sh (which replaces core/).
require "json"
require_relative "message_states"

HOST = "campfire.test"
USER_AGENT = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36"
class GoldenController < ApplicationController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end

def render_with(user:, **options)
  Current.reset
  Current.user = user
  GoldenController.renderer.new(http_host: HOST, https: false, "HTTP_USER_AGENT" => USER_AGENT, "rack.session" => {}).render(**options)
ensure
  Current.reset
end

controller = GoldenController.new
controller.set_request! ActionDispatch::Request.new(Rack::MockRequest.env_for("http://campfire.test/", "rack.session" => {}))
controller.set_response! ActionDispatch::Response.new
view = controller.view_context
Rails.cache = ActiveSupport::Cache::MemoryStore.new
GoldenController.perform_caching = true
labels = JSON.parse(File.read("/seed/labels.json"))
messages = %w[image image_large].map do |name|
  message = Message.with_rendering_details.find(labels.fetch("messages.#{name}"))
  Rails.cache.clear
  html = %w[david@37signals.com jz@37signals.com].to_h do |email|
    [email, render_with(user: User.find_by!(email_address: email), partial: "messages/message", locals: { message: message })]
  end
  { name: name, message: message_view_facts(view, message), html_by_viewer: html }
end

# Change only an in-memory blob attribute; no saves, variant processing or write callbacks.
message = Message.with_rendering_details.find(labels.fetch("messages.image"))
filenames = ["moon.jpg", "black_hole.jpg", "moon.JPG", "archive.tar.gz", "version.1.preview.jpeg",
  "README", ".gitignore", ".hidden.jpg", "a..b", "foo.", "..", "...", "...jpg",
  "dir.with.dots/moon.jpg", "dir/moon.jpg/", "/", "dir\\moon.jpg", "  moon.jpg  ", "lune <&> \"é\".jpg"]
previews = filenames.map do |filename|
  message.attachment.blob.filename = filename
  { raw_filename: filename, content: message_content_facts(view, message), html: view.message_presentation(message) }
end
File.write("/rails/storage/db/image-goldens.json", JSON.pretty_generate(messages: messages, filename_previews: previews) + "\n")
puts "Rails image goldens: #{messages.size} default-seed fixtures, 2 viewers each, #{previews.size} filename previews"
messages.each { |row| puts "Rails image #{row[:name]}: #{row[:html_by_viewer].values.map(&:bytesize).join(', ')} bytes" }
