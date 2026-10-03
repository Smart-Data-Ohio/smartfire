require "test_helper"

class FreshVideoAttachmentTest < ActionDispatch::IntegrationTest
  # Active Storage's deferred uploads must run at real commits, not the
  # simulated commits inside a transactional fixture test.
  self.use_transactional_tests = false

  setup do
    @room = rooms(:watercooler)
    @blob_ids = ActiveStorage::Blob.pluck(:id)
  end

  teardown do
    ActiveStorage::Blob.where.not(id: @blob_ids).find_each(&:delete)
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
  end

  [ :room, :thread ].each do |destination|
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

    message = webhook.send(:create_sync_reply, trigger, attachment: video_upload(:direct))

    assert_equal trigger.id, message.reply_to_message_id
    assert_video_ready(message)
  end

  test "bot API posts a fresh multipart video" do
    post room_bot_messages_url(@room, bot_key_for(users(:bender))),
      params: { attachment: video_upload(:multipart) }

    assert_response :created
    assert_video_ready(@room.messages.order(:id).last)
  end

  private
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
      message.reload
      assert Message.exists?(message.id)
      assert message.attachment.blob.service.exist?(message.attachment.key)
      preview = message.attachment.preview(format: :webp)
      assert preview.image.attached?
      assert preview.image.blob.service.exist?(preview.image.key)
      assert_equal "RIFF", preview.download.byteslice(0, 4)
      assert_equal "WEBP", preview.download.byteslice(8, 4)
    end
end
