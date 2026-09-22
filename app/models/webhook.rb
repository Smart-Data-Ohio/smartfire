require "net/http"
require "uri"
require "restricted_http/private_network_guard"

class Webhook < ApplicationRecord
  ENDPOINT_TIMEOUT = 7.seconds

  belongs_to :user

  # Posts a JSON payload to this webhook's URL through the SSRF guard,
  # pinning the connection to the resolved public address like unfurls.
  # A URL pointing at loopback or a private address raises
  # RestrictedHTTP::Violation instead of posting; a hostname that
  # resolves to nothing raises Surfguard::Unresolvable.
  def post_payload(payload)
    address = RestrictedHTTP::PrivateNetworkGuard.resolve(uri.host)

    Net::HTTP.start(uri.host, uri.port, ipaddr: address, use_ssl: uri.scheme == "https",
      open_timeout: ENDPOINT_TIMEOUT, read_timeout: ENDPOINT_TIMEOUT) do |http|
      request = Net::HTTP::Post.new(uri, "Content-Type" => "application/json")
      request.body = payload
      http.request(request)
    end
  end

  def deliver(message, agent: nil, delivery_id: nil)
    post(payload(message, agent: agent, delivery_id: delivery_id)).tap do |response|
      if text = extract_text_from(response)
        receive_text_reply_to(message.room, text: text)
      elsif attachment = extract_attachment_from(response)
        receive_attachment_reply_to(message.room, attachment: attachment)
      end
    end
  rescue Net::OpenTimeout, Net::ReadTimeout
    receive_text_reply_to message.room, text: "Failed to respond within #{ENDPOINT_TIMEOUT} seconds"
  end

  private
    def post(payload)
      post_payload(payload)
    end

    def uri
      @uri ||= URI(url)
    end

    def payload(message, agent: nil, delivery_id: nil)
      hash = {
        user:    { id: message.creator.id, name: message.creator.name },
        room:    { id: message.room.id, name: message.room.name, path: room_bot_messages_path(message) },
        message: { id: message.id, body: { html: message.body.body, plain: without_recipient_mentions(message.plain_text_body) }, path: message_path(message) }
      }
      if agent
        hash[:agent] = { id: agent.id, name: agent.user.name, owner: agent.owner&.name, delivery_id: delivery_id }
        hash[:pull_request] = Github::PullRequestThread.payload_for_message(message)
        hash[:message][:drive_attachments] = message.drive_attachments.map do |attachment|
          { file_id: attachment.file_id, url: attachment.url }
        end
      end
      hash.to_json
    end

    def message_path(message)
      Rails.application.routes.url_helpers.room_at_message_path(message.room, message)
    end

    def room_bot_messages_path(message)
      Rails.application.routes.url_helpers.room_bot_messages_path(message.room, user.bot_key)
    end

    def extract_text_from(response)
      String.new(response.body).force_encoding("UTF-8") if response.code == "200" && response.content_type.in?(%w[ text/html text/plain ])
    end

    def receive_text_reply_to(room, text:)
      room.messages.create!(body: text, creator: user).broadcast_create
    end

    def extract_attachment_from(response)
      if response.content_type && mime_type = Mime::Type.lookup(response.content_type)
        ActiveStorage::Blob.create_and_upload! \
          io: StringIO.new(response.body), filename: "attachment.#{mime_type.symbol}", content_type: mime_type.to_s
      end
    end

    def receive_attachment_reply_to(room, attachment:)
      room.messages.create_with_attachment!(attachment: attachment, creator: user).broadcast_create
    end

    def without_recipient_mentions(body)
      body \
        .gsub(user.attachable_plain_text_representation(nil), "") # Remove mentions of the recipient user
        .gsub(/\A\p{Space}+|\p{Space}+\z/, "") # Remove leading and trailing whitespace uncluding unicode spaces
    end
end
