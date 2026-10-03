require "test_helper"

class FreshVideoAttachmentTest < ActionDispatch::IntegrationTest
  # Active Storage's deferred uploads must run at real commits, not the
  # simulated commits inside a transactional fixture test.
  self.use_transactional_tests = false

  setup do
    @room = rooms(:watercooler)
    @blob_ids = ActiveStorage::Blob.pluck(:id)
    @message_ids = Message.pluck(:id)
    @thread_ids = ChannelThread.pluck(:id)
    @event_ids = AgentEvent.pluck(:id)
    @session_ids = Session.pluck(:id)
    @membership_state = @room.memberships.pluck(:id, :unread_at)
    @room_updated_at = @room.updated_at
  end

  teardown do
    # Real commits also persist the agent ledger and unread state. Restore
    # them as well as media so later transactional tests see their fixtures.
    AgentEvent.where.not(id: @event_ids).delete_all
    Message.where.not(id: @message_ids).find_each do |message|
      message.importing = true
      message.destroy!
    end
    ChannelThread.where.not(id: @thread_ids).destroy_all
    Session.where.not(id: @session_ids).delete_all
    @legacy_bot&.destroy!
    ActiveStorage::Blob.where.not(id: @blob_ids).find_each(&:purge)
    @membership_state.each { |id, unread_at| Membership.where(id: id).update_all(unread_at: unread_at) }
    Room.where(id: @room.id).update_all(updated_at: @room_updated_at)
  end

  [ :multipart, :direct ].each do |upload|
    test "web thread posts a fresh #{upload} video" do
      sign_in :david
      thread = create_thread

      assert_difference -> { thread.messages.count }, 1 do
        post room_thread_messages_url(@room, thread),
          params: { message: { attachment: video_upload(upload), client_message_id: SecureRandom.uuid } },
          as: :turbo_stream
      end

      assert_response :success
      assert_video_ready(thread.messages.sole)
    end

    test "agent REST thread posts a fresh #{upload} video" do
      thread = create_thread
      params = { thread_id: thread.id,
        message: { attachment: video_upload(upload), client_message_id: SecureRandom.uuid } }
      headers = { "Authorization" => "Bearer bender-test-secret-1234", "Accept" => "application/json" }

      assert_difference -> { thread.messages.count }, 1 do
        post room_agent_messages_url(@room), params: params, headers: headers
      end

      assert_response :created
      assert_video_ready(thread.messages.sole)
    end

    test "web root posts a fresh #{upload} video" do
      sign_in :david
      token = SecureRandom.uuid
      post room_messages_url(@room),
        params: { message: { attachment: video_upload(upload), client_message_id: token } },
        as: :turbo_stream

      assert_response :success
      assert_video_ready(Message.find_by!(client_message_id: token))
    end

    test "agent REST root posts a fresh #{upload} video" do
      token = SecureRandom.uuid
      post room_agent_messages_url(@room),
        params: { message: { attachment: video_upload(upload), client_message_id: token } },
        headers: { "Authorization" => "Bearer bender-test-secret-1234", "Accept" => "application/json" }

      assert_response :created
      assert_video_ready(Message.find_by!(client_message_id: token))
    end
  end

  [ :room, :thread ].each do |destination|
    [ :multipart, :direct ].each do |upload|
      test "replacing a #{destination} attachment with a fresh #{upload} video generates its preview" do
        sign_in :david
        thread = create_thread if destination == :thread
        message = @room.messages.create!(creator: users(:david), thread: thread,
          attachment: fixture_file_upload("moon.jpg", "image/jpeg"))
        url = thread ? room_thread_message_url(@room, thread, message) : room_message_url(@room, message)

        patch url, params: { message: { attachment: video_upload(upload) } },
          headers: { "Accept" => "application/json" }

        assert_response :success
        assert_equal "alpha-centuri.mov", message.reload.attachment.filename.to_s
        assert_video_ready(message)
        assert_rendered_turbo_stream_broadcast thread || @room, :messages,
          action: "replace", target: [ message, :presentation ] do
          assert_select "video.message__attachment[poster]"
        end
      end
    end

    test "forwarding a fresh video to a #{destination} creates its own preview" do
      source = @room.messages.create!(creator: users(:david), body: "A fresh video", attachment: video_upload(:direct))
      target = { room_id: @room.id }
      target[:thread_id] = create_thread.id if destination == :thread

      result = Messages::Forwarder.call(source: source, destinations: [ target ], creator: users(:david)).sole

      assert_not_equal source.attachment.blob_id, result.message.attachment.blob_id
      assert_video_ready(result.message)
    end
  end

  test "a synchronous bot webhook can reply to a thread with a fresh video" do
    thread = create_thread
    trigger = thread.post_message!(creator: users(:david), attributes: { body: "video please" })
    webhook = users(:bender).webhook

    WebMock.stub_request(:post, webhook.url).to_return(status: 200,
      body: file_fixture("alpha-centuri.mov").binread, headers: { "Content-Type" => "video/quicktime" })
    Bot::WebhookJob.perform_now(users(:bender), trigger)
    message = thread.messages.where.not(id: trigger.id).sole

    assert_equal trigger.id, message.reply_to_message_id
    assert_video_ready(message)
  end

  test "bot API posts a fresh multipart video" do
    post room_bot_messages_url(@room, bot_key_for(users(:bender))),
      params: { attachment: video_upload(:multipart) }

    assert_response :created
    assert_video_ready(@room.messages.order(:id).last)
  end

  [ :legacy, :agent ].each do |kind|
    [ :multipart, :direct ].each do |upload|
      test "#{kind} bot replaces an attachment with a fresh #{upload} video" do
        bot = if kind == :legacy
          @legacy_bot = User.create_bot!(name: "Video replacement bot")
          @room.memberships.grant_to(@legacy_bot)
          @legacy_bot
        else
          users(:bender)
        end
        message = @room.messages.create!(creator: bot,
          attachment: fixture_file_upload("moon.jpg", "image/jpeg"))

        patch room_bot_message_url(@room, bot_key_for(bot), message),
          params: { attachment: video_upload(upload) }

        assert_response :success
        assert_video_ready(message)
      end
    end
  end

  test "a completed browser direct upload posts to a thread before any preview exists" do
    sign_in :david
    bytes = file_fixture("alpha-centuri.mov").binread
    post rails_direct_uploads_url, params: { blob: {
      filename: "alpha-centuri.mov", content_type: "video/quicktime",
      byte_size: bytes.bytesize, checksum: Digest::MD5.base64digest(bytes)
    } }, as: :json
    assert_response :success
    upload = response.parsed_body
    put URI(upload.dig("direct_upload", "url")).request_uri,
      params: bytes, headers: upload.dig("direct_upload", "headers")
    assert_response :no_content
    blob = ActiveStorage::Blob.find_signed!(upload["signed_id"])
    assert_equal bytes, blob.download
    assert_not blob.preview_image.attached?

    thread = create_thread
    post room_thread_messages_url(@room, thread),
      params: { message: { attachment: blob.signed_id } }, as: :json

    assert_response :created
    assert_video_ready(thread.messages.sole)
  end

  test "nested transactions process only after the outermost commit and never after rollback" do
    attachment = video_upload(:direct)
    message = nil
    assert_enqueued_jobs 1, only: Message::AttachmentProcessingJob do
      Message.transaction do
        Message.transaction(requires_new: true) do
          message = @room.messages.create_with_attachment!(creator: users(:david), attachment: attachment)
        end
        assert_enqueued_jobs 0, only: Message::AttachmentProcessingJob
        assert_not message.attachment.blob.preview_image.attached?
      end
    end
    assert_video_ready(message)

    assert_no_enqueued_jobs only: Message::AttachmentProcessingJob do
      Message.transaction do
        @room.messages.create_with_attachment!(creator: users(:david), attachment: video_upload(:multipart))
        raise ActiveRecord::Rollback
      end
    end
  end

  test "pending videos render playable source and completion adds a poster and refreshes the thread" do
    sign_in :david
    thread = create_thread
    post room_thread_messages_url(@room, thread),
      params: { message: { attachment: video_upload(:multipart) } }, as: :turbo_stream
    assert_response :success
    message = thread.messages.sole
    assert_not message.attachment.blob.preview_image.attached?
    pending = Nokogiri::HTML.fragment(response.body).at_css("video.message__attachment")
    assert pending["src"].present?
    assert_nil pending["poster"]
    assert_equal "none", pending["preload"]
    stamp = message.updated_at

    assert_video_ready(message)

    assert_operator message.reload.updated_at, :>, stamp
    poster = nil
    assert_rendered_turbo_stream_broadcast thread, :messages, action: "replace", target: [ message, :presentation ] do
      assert_select "video.message__attachment[poster]" do |videos|
        poster = videos.first["poster"]
      end
      assert_select "div[style*='aspect-ratio']"
    end
    # Follow the actual poster URL; it uses the resized WebP representation
    # in the existing view, rather than the unresized eager representation.
    get poster
    assert_response :redirect
    follow_redirect!
    assert_response :success
    assert_equal "image/webp", response.media_type
  end

  test "a processing failure retries without failing or rolling back a thread post" do
    sign_in :david
    thread = create_thread
    post room_thread_messages_url(@room, thread),
      params: { message: { attachment: video_upload(:multipart) } }, headers: { "Accept" => "application/json" }
    assert_response :created
    message = thread.messages.sole
    Message.any_instance.stubs(:process_attachment_now).raises(ActiveStorage::PreviewError, "decoder unavailable")

    perform_enqueued_jobs(only: Message::AttachmentProcessingJob, at: Time.current)

    assert Message.exists?(message.id)
    assert message.attachment.blob.service.exist?(message.attachment.key)
    assert_enqueued_jobs 1, only: Message::AttachmentProcessingJob
  end

  test "a synchronous processing failure does not fail a committed root post" do
    sign_in :david
    Message.any_instance.stubs(:process_attachment_now).raises(ActiveStorage::PreviewError, "decoder unavailable")
    token = SecureRandom.uuid
    post room_messages_url(@room),
      params: { message: { attachment: video_upload(:multipart), client_message_id: token } }, as: :turbo_stream

    assert_response :success
    assert Message.exists?(client_message_id: token)
  end

  test "queued processing discards a deleted message" do
    message = create_thread.post_message!(creator: users(:david), attributes: { attachment: video_upload(:multipart) })
    message.destroy!

    perform_enqueued_jobs(only: Message::AttachmentProcessingJob)

    assert_no_enqueued_jobs only: Message::AttachmentProcessingJob
  end

  [ :raised, :refused ].each do |failure|
    test "rendering recovers a fresh video after a #{failure} enqueue" do
      sign_in :david
      thread = create_thread
      enqueue = Message::AttachmentProcessingJob.stubs(:perform_later)
      failure == :raised ? enqueue.raises(RuntimeError, "queue unavailable") : enqueue.returns(false)
      post room_thread_messages_url(@room, thread),
        params: { message: { attachment: video_upload(:multipart) } },
        headers: { "Accept" => "application/json" }
      assert_response :created
      message = thread.messages.sole
      assert_not message.attachment.blob.preview_image.attached?
      with_message_caching do
        # Cache the posterless fragment while the queue is still unavailable.
        get room_thread_messages_url(@room, thread)
        assert_response :success
        assert_no_enqueued_jobs only: Message::AttachmentProcessingJob
        Message::AttachmentProcessingJob.unstub(:perform_later)

        assert_enqueued_jobs 1, only: Message::AttachmentProcessingJob do
          2.times do
            get room_thread_messages_url(@room, thread)
            assert_response :success
            assert_select "video.message__attachment", 1
            assert_select "video.message__attachment[poster]", 0
          end
        end
        assert_video_ready(message)
      end
    end
  end

  test "a lost job is recovered after its lease expires" do
    sign_in :david
    thread = create_thread
    message = thread.post_message!(creator: users(:david), attributes: { attachment: video_upload(:direct) })
    clear_enqueued_jobs

    travel Message::ATTACHMENT_PROCESSING_LEASE + 1.second do
      assert_enqueued_jobs 1, only: Message::AttachmentProcessingJob do
        2.times do
          get room_thread_messages_url(@room, thread)
          assert_response :success
        end
      end
      assert_video_ready(message)
    end
  end

  test "analysis cannot erase the pending preview claim" do
    sign_in :david
    thread = create_thread
    signed_id = video_upload(:direct)
    analyzer_blob = ActiveStorage::Blob.find_signed!(signed_id)
    analyzer_blob.metadata # Load the metadata before scheduling processing.
    message = thread.post_message!(creator: users(:david), attributes: { attachment: signed_id })

    analyzer_blob.analyze

    assert_no_enqueued_jobs only: Message::AttachmentProcessingJob do
      get room_thread_messages_url(@room, thread)
      assert_response :success
    end
    assert_video_ready(message)
  end

  test "one preview job refreshes both messages sharing a direct upload" do
    signed_id = video_upload(:direct)
    messages = []
    assert_enqueued_jobs 1, only: Message::AttachmentProcessingJob do
      2.times do
        messages << create_thread.post_message!(creator: users(:david), attributes: { attachment: signed_id })
      end
    end

    assert_video_ready(messages.first)

    messages.each do |message|
      assert_rendered_turbo_stream_broadcast message.thread, :messages,
        action: "replace", target: [ message, :presentation ] do
        assert_select "video.message__attachment[poster]"
      end
    end
  end

  test "exhausted processing retries release the claim for rendering recovery" do
    sign_in :david
    thread = create_thread
    message = thread.post_message!(creator: users(:david), attributes: { attachment: video_upload(:direct) })
    Message.any_instance.stubs(:process_attachment_now).raises(ActiveStorage::PreviewError, "decoder unavailable")

    2.times { perform_enqueued_jobs(only: Message::AttachmentProcessingJob) }
    assert_raises ActiveStorage::PreviewError do
      perform_enqueued_jobs(only: Message::AttachmentProcessingJob)
    end
    assert_nil message.attachment.blob.reload.message_processing_token
    Message.any_instance.unstub(:process_attachment_now)

    assert_enqueued_jobs 1, only: Message::AttachmentProcessingJob do
      get room_thread_messages_url(@room, thread)
      assert_response :success
    end
    assert_video_ready(message)
  end

  test "views do not restart exhausted real decoder failures" do
    sign_in :david
    thread = create_thread
    blob = ActiveStorage::Blob.create_and_upload!(io: StringIO.new("not a QuickTime movie"),
      filename: "corrupt.mov", content_type: "video/quicktime")
    decoder_runs = 0
    subscription = ActiveSupport::Notifications.subscribe("preview.active_storage") do |event|
      decoder_runs += 1 if event.payload[:key] == blob.key
    end
    post room_thread_messages_url(@room, thread),
      params: { message: { attachment: blob.signed_id } }, as: :json
    assert_response :created
    message = thread.messages.sole
    2.times { perform_enqueued_jobs(only: Message::AttachmentProcessingJob) }
    assert_raises ActiveStorage::PreviewError do
      perform_enqueued_jobs(only: Message::AttachmentProcessingJob)
    end
    assert_equal 3, decoder_runs

    with_message_caching do
      3.times do
        2.times do
          get room_thread_messages_url(@room, thread)
          assert_response :success
          assert_select "##{ActionView::RecordIdentifier.dom_id(message, :presentation)}" do
            assert_select "video.message__attachment[controls][preload='none']", 1
            assert_select "video[poster], .spinner, [aria-busy='true'], [role='alert']", 0
          end
        end
        # Drain a mistakenly restarted batch too, so this counts real FFmpeg
        # invocations rather than merely observing a queued retry.
        begin
          3.times { perform_enqueued_jobs(only: Message::AttachmentProcessingJob) }
        rescue ActiveStorage::PreviewError
          # Finish checking all views even if they wrongly exhaust a new batch.
        end
      end
    end

    assert_equal 3, decoder_runs, "views must not restart the exhausted decoder budget"
    travel 1.day do
      assert_no_enqueued_jobs only: Message::AttachmentProcessingJob do
        get room_thread_messages_url(@room, thread)
        assert_response :success
      end
    end
    assert Message.exists?(message.id)
    assert_equal "not a QuickTime movie", blob.download
    assert_not blob.reload.preview_image.attached?
  ensure
    ActiveSupport::Notifications.unsubscribe(subscription) if subscription
  end

  [ "ActiveJob::EnqueueError", "RuntimeError" ].each do |error_name|
    test "views back off continuing #{error_name} enqueue refusals and recover afterwards" do
      sign_in :david
      thread = create_thread
      adapter = Message::AttachmentProcessingJob.queue_adapter
      error_class = error_name.constantize
      original_enqueue = adapter.method(:enqueue)
      attempts = 0
      adapter.define_singleton_method(:enqueue) do |job|
        if job.is_a?(Message::AttachmentProcessingJob)
          attempts += 1
          raise error_class, "queue unavailable"
        else
          original_enqueue.call(job)
        end
      end
      post room_thread_messages_url(@room, thread),
        params: { message: { attachment: video_upload(:direct) } }, as: :json
      assert_response :created
      message = thread.messages.sole

      with_message_caching do
        4.times do
          get room_thread_messages_url(@room, thread)
          assert_response :success
          assert_select "video.message__attachment", 1
          assert_select "video[poster]", 0
        end
        assert_equal 1, attempts, "refused enqueues must not repeat on every view"
        2.times do |retry_index|
          travel (retry_index + 1) * (Message::ATTACHMENT_PROCESSING_LEASE + 1.second) do
            3.times { get room_thread_messages_url(@room, thread) }
          end
          assert_equal retry_index + 2, attempts
        end
        adapter.singleton_class.remove_method(:enqueue)

        travel 3 * (Message::ATTACHMENT_PROCESSING_LEASE + 1.second) do
          assert_enqueued_jobs 1, only: Message::AttachmentProcessingJob do
            3.times { get room_thread_messages_url(@room, thread) }
          end
          assert_video_ready(message)
        end
      end
    ensure
      adapter.singleton_class.remove_method(:enqueue) if adapter&.singleton_class&.instance_methods(false)&.include?(:enqueue)
    end
  end

  [ :before_start, :during_decoder ].each do |edit_time|
    test "editing the scheduling message #{edit_time} still refreshes another preview owner" do
      sign_in :david
      signed_id = video_upload(:direct)
      scheduler = create_thread.post_message!(creator: users(:david), attributes: { attachment: signed_id })
      other = create_thread.post_message!(creator: users(:david), attributes: { attachment: signed_id })
      [ scheduler, other ].each(&:broadcast_create)
      blob = other.attachment.blob
      other_stamp = other.updated_at
      scheduler_after_edit = nil
      scheduler_stamp = nil
      edit = -> do
        patch room_thread_message_url(@room, scheduler.thread, scheduler),
          params: { message: { attachment: fixture_file_upload("moon.jpg", "image/jpeg") } },
          headers: { "Accept" => "application/json" }
        assert_response :success
        scheduler_after_edit = find_broadcasts_for(scheduler.thread, :messages)
        scheduler_stamp = Message.find(scheduler.id).updated_at
      end
      if edit_time == :before_start
        edit.call
      else
        subscription = ActiveSupport::Notifications.subscribe("preview.active_storage") do |event|
          edit.call if event.payload[:key] == blob.key
        end
      end

      perform_enqueued_jobs(only: Message::AttachmentProcessingJob)

      assert_equal "moon.jpg", scheduler.reload.attachment.filename.to_s
      if edit_time == :during_decoder
        assert scheduler_after_edit, "the real decoder must reach the edit boundary"
        assert_equal scheduler_after_edit, find_broadcasts_for(scheduler.thread, :messages)
        assert_equal scheduler_stamp, scheduler.updated_at
      end
      assert_operator other.reload.updated_at, :>, other_stamp
      assert_rendered_turbo_stream_broadcast other.thread, :messages,
        action: "replace", target: [ other, :presentation ] do
        assert_select "video.message__attachment[poster]"
      end
      assert_video_ready(other)
    ensure
      ActiveSupport::Notifications.unsubscribe(subscription) if subscription
    end
  end

  test "rendering while the real decoder runs does not start competing processing" do
    sign_in :david
    thread = create_thread
    message = thread.post_message!(creator: users(:david), attributes: { attachment: video_upload(:direct) })
    reached_decoder = false
    subscription = ActiveSupport::Notifications.subscribe("preview.active_storage") do |event|
      next unless event.payload[:key] == message.attachment.key

      reached_decoder = true
      assert_no_enqueued_jobs only: Message::AttachmentProcessingJob do
        get room_thread_messages_url(@room, thread)
        assert_response :success
        assert_select "video.message__attachment[poster]", 0
      end
    end

    assert_video_ready(message)
    assert reached_decoder
  ensure
    ActiveSupport::Notifications.unsubscribe(subscription) if subscription
  end

  test "attachment attach schedules processing and rollback schedules nothing" do
    message = @room.messages.create!(creator: users(:david),
      attachment: fixture_file_upload("moon.jpg", "image/jpeg"))
    assert_no_enqueued_jobs only: Message::AttachmentProcessingJob do
      Message.transaction do
        message.attachment.attach(video_upload(:multipart))
        raise ActiveRecord::Rollback
      end
    end
    assert_equal "moon.jpg", message.reload.attachment.filename.to_s

    message.attachment.attach(video_upload(:multipart))

    assert_video_ready(message)
  end

  test "a job queued for an attachment replaced before it starts is skipped" do
    message = create_thread.post_message!(creator: users(:david), attributes: { attachment: video_upload(:direct) })
    old_blob = message.attachment.blob
    message.update!(attachment: fixture_file_upload("moon.jpg", "image/jpeg"))

    perform_enqueued_jobs(only: Message::AttachmentProcessingJob)

    assert_equal "moon.jpg", message.reload.attachment.filename.to_s
    assert_not old_blob.reload.preview_image.attached?
    assert_nil old_blob.message_processing_token
  end

  test "rendering recovers a video after inline processing failed" do
    sign_in :david
    Message.any_instance.stubs(:process_attachment_now).raises(ActiveStorage::PreviewError, "decoder unavailable")
    token = SecureRandom.uuid
    post room_messages_url(@room),
      params: { message: { attachment: video_upload(:multipart), client_message_id: token } }, as: :turbo_stream
    assert_response :success
    message = Message.find_by!(client_message_id: token)
    assert_not message.attachment.blob.preview_image.attached?
    Message.any_instance.unstub(:process_attachment_now)

    get room_messages_url(@room)

    assert_response :success
    assert_video_ready(message)
  end

  test "a decoder finishing after an attachment edit does not broadcast the old video" do
    sign_in :david
    thread = create_thread
    message = thread.post_message!(creator: users(:david), attributes: { attachment: video_upload(:direct) })
    original = message.attachment.blob
    edited_broadcasts = nil
    edited_stamp = nil
    # Pause at the real FFmpeg boundary, edit through PATCH, then allow the
    # decoder to produce its actual JPEG and WebP. No preview output is stubbed.
    subscription = ActiveSupport::Notifications.subscribe("preview.active_storage") do |event|
      next unless event.payload[:key] == original.key

      patch room_thread_message_url(@room, thread, message),
        params: { message: { attachment: fixture_file_upload("moon.jpg", "image/jpeg") } },
        headers: { "Accept" => "application/json" }
      assert_response :success
      edited_broadcasts = find_broadcasts_for(thread, :messages)
      edited_stamp = Message.find(message.id).updated_at
    end

    perform_enqueued_jobs(only: Message::AttachmentProcessingJob)

    assert edited_broadcasts, "the real decoder must have reached the pause"
    assert original.reload.preview_image.attached?, "the real decoder must finish"
    assert_equal "moon.jpg", message.reload.attachment.filename.to_s
    assert_equal edited_broadcasts, find_broadcasts_for(thread, :messages),
      "completion must not overwrite the edit's image presentation"
    assert_equal edited_stamp, message.updated_at
  ensure
    ActiveSupport::Notifications.unsubscribe(subscription) if subscription
  end

  test "already processed videos remain usable inside a transaction" do
    blob = ActiveStorage::Blob.find_signed!(video_upload(:direct))
    blob.preview(format: :webp).processed
    preview_key = blob.preview_image.key

    message = create_thread.post_message!(creator: users(:david), attributes: { attachment: blob.signed_id })

    assert_video_ready(message)
    assert_equal preview_key, message.attachment.blob.reload.preview_image.key
  end

  test "transactional images and nonrepresentable files retain their attachment behavior" do
    [ [ "moon.jpg", "image/jpeg" ], [ "pixel.bmp", "image/bmp" ] ].each do |filename, content_type|
      message = create_thread.post_message!(creator: users(:david),
        attributes: { attachment: fixture_file_upload(filename, content_type) })
      perform_enqueued_jobs(only: Message::AttachmentProcessingJob)
      assert message.reload.attachment.attached?
      assert_equal file_fixture(filename).binread, message.attachment.download
      assert_predicate message.attachment.blob, :analyzed?
      if message.attachment.representable?
        representation = message.attachment.representation(:thumb)
        variant = message.attachment.blob.variant_records.find_by!(variation_digest: representation.variation.digest)
        assert variant.image.blob.service.exist?(variant.image.key)
      else
        assert_not message.attachment.blob.preview_image.attached?
      end
    end
  end

  private
    def with_message_caching
      original_cache = Rails.cache
      original_collection_cache = ActionView::PartialRenderer.collection_cache
      original_perform_caching = ActionController::Base.perform_caching
      Rails.cache = ActiveSupport::Cache::MemoryStore.new
      ActionView::PartialRenderer.collection_cache = Rails.cache
      ActionController::Base.perform_caching = true
      yield
    ensure
      ActionController::Base.perform_caching = original_perform_caching
      ActionView::PartialRenderer.collection_cache = original_collection_cache
      Rails.cache = original_cache
    end

    def create_thread
      ChannelThread.create!(room: @room, creator: users(:david), name: "Fresh video", closed_at: Time.current)
    end

    def video_upload(upload)
      if upload == :multipart
        fixture_file_upload("alpha-centuri.mov", "video/quicktime")
      else
        # The source is uploaded outside the posting transaction and has no
        # preview, exactly as with a completed browser direct upload.
        File.open(file_fixture("alpha-centuri.mov")) do |io|
          blob = ActiveStorage::Blob.create_and_upload!(io: io,
            filename: "alpha-centuri.mov", content_type: "video/quicktime")
          assert blob.service.exist?(blob.key)
          assert_not blob.preview_image.attached?
          blob.signed_id
        end
      end
    end

    def assert_video_ready(message)
      perform_enqueued_jobs(only: Message::AttachmentProcessingJob)
      message.reload
      assert Message.exists?(message.id)
      assert message.attachment.blob.service.exist?(message.attachment.key)
      preview = message.attachment.preview(format: :webp)
      assert preview.image.attached?
      assert preview.image.blob.service.exist?(preview.image.key)
      variant = preview.image.blob.variant_records.find_by!(variation_digest: preview.variation.digest)
      assert variant.image.blob.service.exist?(variant.image.key)
      assert_equal "image/webp", variant.image.content_type
      assert_equal "RIFF", preview.download.byteslice(0, 4)
      assert_equal "WEBP", preview.download.byteslice(8, 4)
    end
end
