require "test_helper"

class MessagesControllerTest < ActionDispatch::IntegrationTest
  setup do
    host! "smartfire.test"

    sign_in :david
    @room = rooms(:watercooler)
    @messages = @room.messages.ordered.to_a
  end

  test "index returns the last page by default" do
    get room_messages_url(@room)

    assert_response :success
    ensure_messages_present @messages.last
  end

  test "index is not found for a soft-deleted room" do
    @room.update_columns(deleted_at: Time.current)

    assert_raises ActiveRecord::RecordNotFound do
      get room_messages_url(@room)
    end
  end

  test "index returns a page before the specified message" do
    get room_messages_url(@room, before: @messages.third)

    assert_response :success
    ensure_messages_present @messages.first, @messages.second
    ensure_messages_not_present @messages.third, @messages.fourth, @messages.fifth
  end

  test "index returns a page after the specified message" do
    get room_messages_url(@room, after: @messages.third)

    assert_response :success
    ensure_messages_present @messages.fourth, @messages.fifth
    ensure_messages_not_present @messages.first, @messages.second, @messages.third
  end

  test "index returns no_content when there are no messages" do
    @room.messages.destroy_all

    get room_messages_url(@room)

    assert_response :no_content
  end

  test "index etag changes when an off-page reply source is edited" do
    source = @room.messages.create!(creator: users(:david), markdown_source: "etag source", client_message_id: "etag-source")
    40.times do |index|
      @room.messages.create!(creator: users(:david), markdown_source: "etag filler #{index}", client_message_id: "etag-filler-#{index}")
    end
    reply = @room.messages.create!(
      creator: users(:david), markdown_source: "etag reply", reply_to_message: source, client_message_id: "etag-reply"
    )

    get room_messages_url(@room)
    assert_response :success
    assert_select "##{dom_id(reply)}", count: 1
    assert_select "##{dom_id(source)}", count: 0
    etag = response.headers["ETag"]

    get room_messages_url(@room), headers: { "If-None-Match" => etag }
    assert_response :not_modified

    put room_message_url(@room, source), params: { message: { markdown_source: "etag source edited" } }

    get room_messages_url(@room), headers: { "If-None-Match" => etag }
    assert_response :success
  end

  test "index etag changes when a card fetch completes" do
    pull_request = Github::PullRequest.for_reference(owner: "rails", repo: "rails", number: 520)
    pull_request.update!(private: false, title: "Before fetch", state: "open", fetched_at: Time.current)
    @room.messages.create!(
      creator: users(:david), markdown_source: "see https://github.com/rails/rails/pull/520", client_message_id: "etag-card"
    )

    get room_messages_url(@room)
    assert_response :success
    etag = response.headers["ETag"]

    get room_messages_url(@room), headers: { "If-None-Match" => etag }
    assert_response :not_modified

    pull_request.update!(title: "After fetch", fetched_at: Time.current)

    get room_messages_url(@room), headers: { "If-None-Match" => etag }
    assert_response :success
  end

  test "index etag changes when an author is renamed" do
    @room.messages.create!(creator: users(:jason), markdown_source: "rename me", client_message_id: "etag-rename")

    get room_messages_url(@room)
    assert_response :success
    etag = response.headers["ETag"]

    get room_messages_url(@room), headers: { "If-None-Match" => etag }
    assert_response :not_modified

    users(:jason).update!(name: "Jason Renamed")

    get room_messages_url(@room), headers: { "If-None-Match" => etag }
    assert_response :success
  end

  test "get renders a single message belonging to the user" do
    message = @room.messages.where(creator: users(:david)).first

    get room_message_url(@room, message)

    assert_response :success
  end

  test "room message list announces live appends" do
    get room_url(@room)

    assert_response :success
    assert_select "##{dom_id(@room, :messages)}[role='log'][aria-live='polite'][aria-relevant='additions']", 1
  end

  test "image attachments use the filename as alt text" do
    post room_messages_url(@room, format: :turbo_stream), params: {
      message: { attachment: fixture_file_upload("moon.jpg", "image/jpeg"), client_message_id: "alt-text-1" }
    }
    assert_response :success

    get room_url(@room)
    assert_response :success
    assert_select "img.message__attachment[alt='moon']", 1
  end

  test "creating a message broadcasts the message to the room" do
    post room_messages_url(@room, format: :turbo_stream), params: { message: { body: "New one", client_message_id: 999 } }

    assert_rendered_turbo_stream_broadcast @room, :messages, action: "append", target: [ @room, :messages ] do
      assert_select ".message__body", text: /New one/
      assert_copy_link_button room_at_message_url(@room, Message.last, host: "smartfire.test")
    end
  end

  test "broadcast message actions preserve a nonstandard request port" do
    origin = "http://smartfire.test:3443"
    post "#{origin}#{room_messages_path(@room, format: :turbo_stream)}", params: {
      message: { markdown_source: "A live message", client_message_id: "broadcast-port" }
    }

    assert_rendered_turbo_stream_broadcast @room, :messages, action: "append", target: [ @room, :messages ] do
      assert_select "[data-actions-url='#{origin}#{actions_room_message_path(@room, Message.last)}']"
      assert_copy_link_button "#{origin}#{room_at_message_path(@room, Message.last)}"
    end
  end

  test "creating a Markdown message preserves its source and derives the rich body" do
    source = "# Release\n\n**Ready**"

    post room_messages_url(@room, format: :turbo_stream), params: {
      message: { body: "<script>wrong source</script>", markdown_source: source, client_message_id: 999 }
    }

    message = Message.last
    assert_response :success
    assert_equal source, message.markdown_source
    assert_equal "Release\n\nReady", message.plain_text_body
    assert_match %r{<h1>Release</h1>}, message.body.body.to_html
    assert_no_match /script/, message.body.body.to_html
  end

  test "preview renders the same safe Markdown without writing" do
    assert_no_difference -> { Message.count } do
      post preview_room_messages_url(@room), params: {
        message: { markdown_source: "## Preview\n\n<script>x</script>\n\n- [x] @[David]" }
      }, as: :json
    end

    assert_response :success
    html = response.parsed_body.fetch("html")
    assert_match %r{<h2>Preview</h2>}, html
    assert_match %r{<input type="checkbox" checked="" disabled="disabled">}, html
    assert_match %r{<div class="mention mention--user-#{users(:david).id}"}, html
    assert_match %r{data-user-id="#{users(:david).id}"}, html
    assert_no_match /<script/, html
  end

  test "preview requires room membership" do
    sign_in :kevin

    assert_raises ActiveRecord::RecordNotFound do
      post preview_room_messages_url(@room), params: { message: { markdown_source: "No access" } }
    end
  end

  test "preview rejects oversized Markdown without parsing or writing it" do
    assert_no_difference -> { Message.count } do
      post preview_room_messages_url(@room), params: {
        message: { markdown_source: "x" * (Message::Markdown::SOURCE_LIMIT + 1) }
      }
    end

    assert_response :unprocessable_content
    assert_match(/limited/, response.parsed_body.fetch("error"))
  end

  test "preview is protected against cross-site form submissions" do
    original_forgery_protection = ActionController::Base.allow_forgery_protection
    ActionController::Base.allow_forgery_protection = true

    assert_raises ActionController::InvalidAuthenticityToken do
      post preview_room_messages_url(@room), params: { message: { markdown_source: "No token" } }
    end
  ensure
    ActionController::Base.allow_forgery_protection = original_forgery_protection
  end

  test "creating a message broadcasts unread room to each member" do
    @room.users.each do |member|
      assert_broadcasts UnreadRoomsChannel.stream_name_for(member.id), 1 do
        post room_messages_url(@room, format: :turbo_stream), params: { message: { body: "New one #{member.id}", client_message_id: member.id } }
      end
    end
  end

  test "creating a message doesn't broadcast unread room to non-members" do
    outsiders = User.where.not(id: @room.users.map(&:id))
    assert outsiders.any?, "need someone outside the room for this test to mean anything"

    outsiders.each do |outsider|
      assert_no_broadcasts UnreadRoomsChannel.stream_name_for(outsider.id) do
        post room_messages_url(@room, format: :turbo_stream), params: { message: { body: "New one", client_message_id: 999 } }
      end
    end
  end

  test "update updates a message belonging to the user" do
    message = @room.messages.where(creator: users(:david)).first

    Turbo::StreamsChannel.expects(:broadcast_replace_to).times(4)  # presentation plus meta plus both card containers
    put room_message_url(@room, message), params: { message: { body: "Updated body" } }

    assert_redirected_to room_message_url(@room, message)
    assert_equal "Updated body", message.reload.plain_text_body
  end

  test "updating a Markdown message preserves exact new source" do
    message = @room.messages.create!(creator: users(:david), markdown_source: "**Before**", client_message_id: "markdown-update")
    source = "## After\n\n`code`"

    Turbo::StreamsChannel.expects(:broadcast_replace_to).times(4)  # presentation plus meta plus both card containers
    put room_message_url(@room, message), params: { message: { markdown_source: source } }

    assert_redirected_to room_message_url(@room, message)
    assert_equal source, message.reload.markdown_source
    assert_equal "After\n\ncode", message.plain_text_body
  end

  test "a legacy body update clears stale Markdown mode" do
    message = @room.messages.create!(creator: users(:david), markdown_source: "**Before**", client_message_id: "markdown-to-rich")

    Turbo::StreamsChannel.expects(:broadcast_replace_to).times(4)  # presentation plus meta plus both card containers
    put room_message_url(@room, message), params: { message: { body: "Legacy again" } }

    assert_redirected_to room_message_url(@room, message)
    assert_nil message.reload.markdown_source
    assert_equal "Legacy again", message.plain_text_body
  end

  test "editing a message to add a PR URL broadcasts the new card" do
    pull_request = Github::PullRequest.for_reference(owner: "rails", repo: "rails", number: 510)
    pull_request.update!(private: false, title: "Seeded PR card", state: "open", fetched_at: Time.current)
    message = @room.messages.create!(creator: users(:david), markdown_source: "no links here", client_message_id: "card-add")

    put room_message_url(@room, message), params: { message: { markdown_source: "see https://github.com/rails/rails/pull/510" } }

    assert_redirected_to room_message_url(@room, message)
    assert_equal [ pull_request ], message.reload.github_pull_requests
    assert_rendered_turbo_stream_broadcast @room, :messages, action: "replace", target: [ message, :github_pr_cards ] do
      assert_select ".github-pr-card", text: /Seeded PR card/
    end
  end

  test "editing a message to remove a PR URL broadcasts an empty card container" do
    pull_request = Github::PullRequest.for_reference(owner: "rails", repo: "rails", number: 511)
    pull_request.update!(private: false, title: "Removed PR card", state: "open", fetched_at: Time.current)
    message = @room.messages.create!(
      creator: users(:david), markdown_source: "see https://github.com/rails/rails/pull/511", client_message_id: "card-remove"
    )
    assert_equal [ pull_request ], message.github_pull_requests

    put room_message_url(@room, message), params: { message: { markdown_source: "never mind" } }

    assert_redirected_to room_message_url(@room, message)
    assert_empty message.reload.github_pull_requests
    assert_rendered_turbo_stream_broadcast @room, :messages, action: "replace", target: [ message, :github_pr_cards ] do
      assert_select ".github-pr-card", count: 0
      assert_select "turbo-frame", count: 0
    end
  end

  test "legacy rich-text edits re-sync card references" do
    room = rooms(:designers)
    event = events(:launch_party)
    message = room.messages.create!(creator: users(:david), body: "<div>no links here</div>", client_message_id: "legacy-resync")

    put room_message_url(room, message), params: {
      message: { body: "<div>see https://github.com/rails/rails/pull/512 and https://x.com/jack/status/424242 and /rooms/#{room.id}/events/#{event.id}</div>" }
    }

    assert_redirected_to room_message_url(room, message)
    assert_equal [ [ "rails", "rails", 512 ] ], message.reload.github_pull_requests.map { |pr| [ pr.owner, pr.repo, pr.number ] }
    assert_equal [ "424242" ], message.twitter_posts.map(&:post_id)
    assert_equal [ event ], message.events
  end

  test "messages render empty card containers for future broadcasts" do
    message = @room.messages.create!(creator: users(:david), markdown_source: "no links", client_message_id: "empty-cards")

    get room_message_url(@room, message)

    assert_response :success
    assert_select "##{dom_id(message, :github_pr_cards)}.github-pr-cards"
    assert_select "##{dom_id(message, :twitter_cards)}.x-post-cards"
  end

  test "admin cannot update a message belonging to another user" do
    message = @room.messages.where(creator: users(:jason)).first

    assert_no_changes -> { message.reload.plain_text_body } do
      put room_message_url(@room, message), params: { message: { body: "Updated body" } }
    end

    assert_response :forbidden
  end

  test "destroy destroys a message belonging to the user" do
    message = @room.messages.where(creator: users(:david)).first

    assert_difference -> { Message.count }, -1 do
      Turbo::StreamsChannel.expects(:broadcast_remove_to).once
      delete room_message_url(@room, message, format: :turbo_stream)
      assert_response :success
    end
  end

  test "admin destroy destroys a message belonging to another user" do
    assert users(:david).administrator?
    message = @room.messages.where(creator: users(:jason)).first

    assert_difference -> { Message.count }, -1 do
      Turbo::StreamsChannel.expects(:broadcast_remove_to).once
      delete room_message_url(@room, message, format: :turbo_stream)
      assert_response :success
    end
  end

  test "destroy broadcasts tombstone updates to replies" do
    source = @room.messages.where(creator: users(:david)).first
    reply = @room.messages.create!(
      creator: users(:david), markdown_source: "a reply", reply_to_message: source, client_message_id: "tombstone-reply"
    )

    delete room_message_url(@room, source, format: :turbo_stream)

    assert_response :success
    assert_rendered_turbo_stream_broadcast @room, :messages, action: "remove", target: source
    assert_rendered_turbo_stream_broadcast @room, :messages, action: "replace", target: reply do
      assert_select ".message__reply-preview", text: /Replying to a deleted message/
    end
  end

  test "destroying a thread parent broadcasts a thread summary refresh" do
    parent = @room.messages.where(creator: users(:david)).first
    thread = ChannelThread.create!(room: @room, creator: users(:david), name: "Parented thread", parent_message: parent)

    delete room_message_url(@room, parent, format: :turbo_stream)

    assert_response :success
    refreshes = thread_summary_refreshes_for(users(:david), thread)
    assert_equal 1, refreshes.size
    assert_equal @room.id, refreshes.sole["roomId"]
  end

  test "edited messages show an edited marker with the edit time" do
    message = @room.messages.create!(creator: users(:david), markdown_source: "original", client_message_id: "edited-marker")

    get room_message_url(@room, message)
    assert_response :success
    assert_select ".message__edited", count: 0

    put room_message_url(@room, message), params: { message: { markdown_source: "edited" } }

    get room_message_url(@room, message)
    assert_response :success
    assert_select "time.message__edited[data-local-time-target='title'][datetime]", text: "(edited)"
    assert_select ".message__edited[title^='Edited ']"
  end

  test "editing a message broadcasts its meta so other clients see the edited marker" do
    message = @room.messages.create!(creator: users(:david), markdown_source: "original", client_message_id: "edited-meta")

    put room_message_url(@room, message), params: { message: { markdown_source: "edited" } }

    assert_redirected_to room_message_url(@room, message)
    assert_rendered_turbo_stream_broadcast @room, :messages, action: "replace", target: [ message, :meta ] do
      assert_select ".message__edited", text: "(edited)"
    end
  end

  test "identical and attachment-only saves do not mark a message edited" do
    message = @room.messages.create!(creator: users(:david), markdown_source: "stays the same", client_message_id: "edited-identical")

    put room_message_url(@room, message), params: { message: { markdown_source: "stays the same" } }
    assert_nil message.reload.edited_at

    put room_message_url(@room, message), params: {
      message: { markdown_source: "stays the same", attachment: fixture_file_upload("moon.jpg", "image/jpeg") }
    }
    assert_nil message.reload.edited_at

    get room_message_url(@room, message)
    assert_response :success
    assert_select ".message__edited", count: 0
  end

  test "identical rich-text saves do not mark a message edited" do
    message = @room.messages.create!(creator: users(:david), body: "<div>legacy text</div>", client_message_id: "edited-identical-rich")

    put room_message_url(@room, message), params: { message: { body: message.body.body.to_html } }

    assert_nil message.reload.edited_at
  end

  test "a blank body on a bodyless message does not mark it edited" do
    message = @room.messages.create!(creator: users(:david), client_message_id: "edited-bodyless")
    message.attachment.attach(fixture_file_upload("moon.jpg", "image/jpeg"))

    put room_message_url(@room, message), params: {
      message: { body: "", attachment: fixture_file_upload("earth.png", "image/png") }
    }

    assert_nil message.reload.edited_at
  end

  test "formatting-only rich-text edits still mark a message edited" do
    message = @room.messages.create!(creator: users(:david), body: "<div>legacy text</div>", client_message_id: "edited-format")

    put room_message_url(@room, message), params: { message: { body: "<div><strong>legacy text</strong></div>" } }

    assert_not_nil message.reload.edited_at
  end

  test "reactions do not mark a message edited" do
    message = @room.messages.create!(
      creator: users(:david), markdown_source: "react to this", client_message_id: "edited-reaction",
      created_at: 1.hour.ago, updated_at: 1.hour.ago
    )
    Boost.create!(message:, booster: users(:jason), content: "👍")

    assert_operator message.reload.updated_at, :>, message.created_at

    get room_message_url(@room, message)
    assert_response :success
    assert_select ".message__edited", count: 0
  end

  test "card fetches do not mark a message edited" do
    pull_request = Github::PullRequest.for_reference(owner: "rails", repo: "rails", number: 530)
    message = @room.messages.create!(
      creator: users(:david), markdown_source: "see https://github.com/rails/rails/pull/530",
      client_message_id: "edited-card", created_at: 1.hour.ago, updated_at: 1.hour.ago
    )

    pull_request.update!(private: false, title: "Fetched", fetched_at: Time.current)

    get room_message_url(@room, message)
    assert_response :success
    assert_select ".message__edited", count: 0
  end

  test "reply tombstones do not mark the reply edited" do
    source = @room.messages.create!(creator: users(:david), markdown_source: "source", client_message_id: "edited-tombstone-source")
    reply = @room.messages.create!(
      creator: users(:david), markdown_source: "reply", reply_to_message: source,
      client_message_id: "edited-tombstone-reply", created_at: 1.hour.ago, updated_at: 1.hour.ago
    )

    delete room_message_url(@room, source, format: :turbo_stream)

    assert_operator reply.reload.updated_at, :>, reply.created_at

    get room_message_url(@room, reply)
    assert_response :success
    assert_select ".message__edited", count: 0
  end

  test "ensure non-admin can't update a message belonging to another user" do
    sign_in :jz
    assert_not users(:jz).administrator?

    room = rooms(:designers)
    message = room.messages.where(creator: users(:jason)).first

    put room_message_url(room, message), params: { message: { body: "Updated body" } }
    assert_response :forbidden
  end

  test "ensure non-admin can't destroy a message belonging to another user" do
    sign_in :jz
    assert_not users(:jz).administrator?

    room = rooms(:designers)
    message = room.messages.where(creator: users(:jason)).first

    delete room_message_url(room, message, format: :turbo_stream)
    assert_response :forbidden
  end

  test "mentioning a bot triggers a webhook" do
    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200)

    assert_enqueued_jobs 1, only: Agent::DeliveryJob do
      post room_messages_url(@room, format: :turbo_stream), params: { message: {
        body: "<div>Hey #{mention_attachment_for(:bender)}</div>", client_message_id: 999 } }
    end
  end

  test "mentioning a bot from Markdown triggers a webhook" do
    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200)

    assert_enqueued_jobs 1, only: Agent::DeliveryJob do
      post room_messages_url(@room, format: :turbo_stream), params: { message: {
        markdown_source: "Hey @[Bender Bot]", client_message_id: 999 } }
    end
  end

  test "mentioning an agent-backed bot skips the legacy webhook job" do
    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200)

    assert_no_enqueued_jobs only: Bot::WebhookJob do
      post room_messages_url(@room, format: :turbo_stream), params: { message: {
        markdown_source: "Hey @[Bender Bot]", client_message_id: "agent-only" } }
    end
  end

  test "mentioning an agent-backed bot posts exactly one webhook with the agent key" do
    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200)

    post room_messages_url(@room, format: :turbo_stream), params: { message: {
      markdown_source: "Hey @[Bender Bot]", client_message_id: "agent-once" } }

    perform_enqueued_jobs only: Agent::DeliveryJob
    perform_enqueued_jobs only: Agent::EventWebhookJob
    perform_enqueued_jobs only: Bot::WebhookJob

    assert_requested :post, webhooks(:bender).url, times: 1
    assert_requested :post, webhooks(:bender).url,
      body: hash_including("agent" => hash_including("id" => agents(:bender_agent).id)), times: 1
  end

  test "revoked agent delivery posts no webhook" do
    WebMock.stub_request(:post, webhooks(:bender).url).to_return(status: 200)
    grant = AgentGrant.create!(agent: agents(:bender_agent), room: @room, granted_by: users(:david), capability: "read_messages")

    post room_messages_url(@room, format: :turbo_stream), params: { message: {
      markdown_source: "Hey @[Bender Bot]", client_message_id: "agent-revoked" } }

    grant.revoke!
    perform_enqueued_jobs only: Agent::DeliveryJob
    perform_enqueued_jobs only: Agent::EventWebhookJob
    perform_enqueued_jobs only: Bot::WebhookJob

    assert_not_requested :post, webhooks(:bender).url
  end

  test "mentioning a bot without an agent row still uses the legacy webhook" do
    bot = User.create_bot!(name: "Legacy Bot", webhook_url: "https://example.test/legacy-hook")
    @room.memberships.grant_to(bot)
    WebMock.stub_request(:post, bot.webhook.url).to_return(status: 200)

    assert_enqueued_jobs 1, only: Bot::WebhookJob do
      assert_no_enqueued_jobs only: Agent::DeliveryJob do
        post room_messages_url(@room, format: :turbo_stream), params: { message: {
          markdown_source: "Hey @[Legacy Bot]", client_message_id: "legacy-only" } }
      end
    end

    perform_enqueued_jobs only: Agent::DeliveryJob
    perform_enqueued_jobs only: Bot::WebhookJob

    assert_requested :post, bot.webhook.url, body: hash_excluding("agent"), times: 1
  end

  test "retried create with the same client id returns the original message" do
    params = { message: { markdown_source: "Post once", client_message_id: "retry-root-post" } }

    assert_difference -> { @room.messages.count }, 1 do
      post room_messages_url(@room, format: :turbo_stream), params: params
      assert_response :success
    end
    original_id = @room.messages.order(:id).last.id

    assert_no_difference -> { @room.messages.count } do
      post room_messages_url(@room, format: :turbo_stream), params: params
      assert_response :success
    end

    assert_equal original_id, @room.messages.order(:id).last.id
    assert_equal "retry-root-post", @room.messages.order(:id).last.client_message_id
  end

  private
    def thread_summary_refreshes_for(user, thread)
      ActionCable.server.pubsub.broadcasts(UnreadThreadsChannel.stream_name_for(user.id))
        .map { |broadcast| JSON.parse(broadcast) }
        .select { |payload| payload["threadId"] == thread.id && payload["refreshOnly"] == true }
    end

    def ensure_messages_present(*messages, count: 1)
      messages.each do |message|
        assert_select "#" + dom_id(message), count:
      end
    end

    def ensure_messages_not_present(*messages)
      ensure_messages_present *messages, count: 0
    end

    def assert_copy_link_button(url)
      assert_select "a.message__permalink[href='#{url}']"
    end
end
