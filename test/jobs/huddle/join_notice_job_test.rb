require "test_helper"

class Huddle::JoinNoticeJobTest < ActiveSupport::TestCase
  include ActiveJob::TestHelper

  setup do
    @original_api_secret = ENV["LIVEKIT_API_SECRET"]
    ENV["LIVEKIT_API_SECRET"] = "test-api-secret"
    @room = rooms(:david_and_jason)
  end

  teardown do
    ENV["LIVEKIT_API_SECRET"] = @original_api_secret
  end

  test "a first sighting enqueues the join notice alongside the presence broadcast" do
    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))

    assert_enqueued_with(job: Huddle::JoinNoticeJob, args: [ grant.id ]) do
      assert_enqueued_with(job: Huddle::BroadcastPresenceJob, args: [ grant.id ]) do
        grant.record_seen!
      end
    end
  end

  test "a repeat sighting enqueues no join notice" do
    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))
    perform_enqueued_jobs only: [ Huddle::JoinNoticeJob, Huddle::BroadcastPresenceJob ] do
      grant.record_seen!
    end

    assert_no_enqueued_jobs only: Huddle::JoinNoticeJob do
      travel 11.seconds do
        grant.record_seen!
      end
    end
  end

  test "performing the job notifies the room's members" do
    grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))
    grant.update_columns(last_seen_at: Time.current)
    ActivityItem.where(user: users(:jason), event_type: "huddle_started").update_all(event_type: "huddle_missed")

    Rails.configuration.x.web_push_pool.expects(:queue).once
    assert_broadcasts HuddleNoticeChannel.stream_name_for(users(:jason).id), 1 do
      Huddle::JoinNoticeJob.perform_now(grant.id)
    end
  end

  test "a missing grant is ignored" do
    Rails.configuration.x.web_push_pool.expects(:queue).never

    Huddle::JoinNoticeJob.perform_now(0)
  end
end
