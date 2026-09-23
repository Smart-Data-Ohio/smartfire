require "test_helper"

class Huddle::JoinPusherTest < ActiveSupport::TestCase
  setup do
    @original_api_secret = ENV["LIVEKIT_API_SECRET"]
    ENV["LIVEKIT_API_SECRET"] = "test-api-secret"
    @room = rooms(:david_and_jason)
    @grant = HuddleGrant.issue!(session: sessions(:david_safari), membership: memberships(:david_david_and_jason))
    @membership = memberships(:jason_david_and_jason)
    @pool = Rails.configuration.x.web_push_pool
  end

  teardown do
    ENV["LIVEKIT_API_SECRET"] = @original_api_secret
  end

  test "pushes the join to the recipient's subscriptions and stamps the throttle" do
    @pool.expects(:queue).once.with do |payload, subscriptions|
      assert_equal "David joined your huddle", payload[:title]
      assert_equal Rails.application.routes.url_helpers.room_path(@room), payload[:path]
      assert_equal "huddle-#{@room.id}", payload[:tag]
      assert_equal [ users(:jason).id ], subscriptions.order(:id).pluck(:user_id)
      true
    end

    push!

    assert @membership.reload.last_huddle_join_push_at.present?
  end

  test "a second push inside ten minutes is throttled" do
    @pool.expects(:queue).once
    push!
    pushed_at = @membership.reload.last_huddle_join_push_at

    @pool.expects(:queue).never
    push!

    assert_equal pushed_at, @membership.reload.last_huddle_join_push_at
  end

  test "a push ten minutes later goes out again" do
    @pool.expects(:queue).once
    push!

    travel 11.minutes do
      @pool.expects(:queue).once
      push!
    end
  end

  test "a DND recipient gets no push and burns no throttle window" do
    users(:jason).update!(dnd_enabled: true)

    @pool.expects(:queue).never
    push!

    assert_nil @membership.reload.last_huddle_join_push_at
  end

  test "a starred joiner still pushes through DND" do
    users(:jason).update!(dnd_enabled: true)
    DndAllowedUser.create!(user: users(:jason), allowed_user: users(:david))

    @pool.expects(:queue).once
    push!
  end

  test "a recipient in quiet hours gets no push" do
    travel_to Time.zone.parse("2026-09-23 12:00") do
      users(:jason).update!(quiet_hours_enabled: true, quiet_hours_start: "09:00", quiet_hours_end: "17:00")

      @pool.expects(:queue).never
      push!
    end
  end

  test "a recipient quiet in a meeting gets no push" do
    users(:jason).update!(meeting_status_enabled: true, meeting_dnd_enabled: true)
    Calendar::MeetingCache.create!(user: users(:jason), fetched_at: Time.current,
      busy_intervals: [ [ 5.minutes.ago.iso8601, 55.minutes.from_now.iso8601 ] ])

    @pool.expects(:queue).never
    push!
  end

  test "an out-of-office recipient gets no push unless they keep notifications on" do
    users(:jason).update!(ooo_until: 1.day.from_now)

    @pool.expects(:queue).never
    push!

    users(:jason).update!(ooo_notify_enabled: true)

    @pool.expects(:queue).once
    push!
  end

  test "a connected recipient gets no push and burns no throttle window" do
    @membership.connected

    @pool.expects(:queue).never
    push!

    assert_nil @membership.reload.last_huddle_join_push_at
  end

  test "a switched-off or hidden room gets no push" do
    @membership.update!(involvement: "nothing")

    @pool.expects(:queue).never
    push!

    @membership.update!(involvement: "invisible")
    push!
  end

  test "a muted room gets no push and burns no throttle window" do
    @membership.update!(involvement: "muted")

    @pool.expects(:queue).never
    push!

    assert_nil @membership.reload.last_huddle_join_push_at
  end

  test "a recipient with huddle invitations switched off gets no push and burns no throttle window" do
    users(:jason).update!(inbox_preferences: { "huddle_invitations" => false })

    @pool.expects(:queue).never
    push!

    assert_nil @membership.reload.last_huddle_join_push_at
  end

  test "a recipient with no subscriptions burns no throttle window" do
    Push::Subscription.where(user: users(:jason)).destroy_all

    @pool.expects(:queue).never
    push!

    assert_nil @membership.reload.last_huddle_join_push_at
  end

  private
    def push!
      Huddle::JoinPusher.new(grant: @grant, recipient: users(:jason), room_membership: @membership).push
    end
end
