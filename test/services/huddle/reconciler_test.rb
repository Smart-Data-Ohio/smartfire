require "test_helper"

class Huddle::ReconcilerTest < ActiveSupport::TestCase
  setup do
    @original_environment = ENV.values_at("LIVEKIT_INTERNAL_URL", "LIVEKIT_API_KEY", "LIVEKIT_API_SECRET")
    ENV["LIVEKIT_INTERNAL_URL"] = "http://livekit.example.test:7880"
    ENV["LIVEKIT_API_KEY"] = "test-api-key"
    ENV["LIVEKIT_API_SECRET"] = "test-api-secret"
  end

  teardown do
    %w[ LIVEKIT_INTERNAL_URL LIVEKIT_API_KEY LIVEKIT_API_SECRET ].zip(@original_environment).each do |name, value|
      ENV[name] = value
    end
  end

  test "one pass resolves overdue invitations, ends stale streams, and reconciles cleanup" do
    Huddle::InvitationResolver.expects(:resolve_overdue!).once.with
    Stream.expects(:end_stale_live!).once.with
    HuddleCleanup.expects(:reconcile_now).once.returns(0)

    Huddle::Reconciler.new.reconcile_once
  end

  test "a resolver failure is logged and does not stop cleanup reconciliation" do
    Huddle::InvitationResolver.stubs(:resolve_overdue!).raises(StandardError, "boom")
    Rails.logger.expects(:error).with("Huddle invitation resolution failed: StandardError")
    HuddleCleanup.expects(:reconcile_now).once.returns(0)

    Huddle::Reconciler.new.reconcile_once
  end

  test "a stale-stream failure is logged and does not stop cleanup reconciliation" do
    Stream.stubs(:end_stale_live!).raises(StandardError, "boom")
    Rails.logger.expects(:error).with("Huddle stream reconciliation failed: StandardError")
    HuddleCleanup.expects(:reconcile_now).once.returns(0)

    Huddle::Reconciler.new.reconcile_once
  end

  test "one pass ends a quiet presenter's live stream" do
    room = Rooms::Stage.create_for({ name: "Town Hall", creator: users(:david) }, users: [ users(:david) ])
    membership = room.memberships.find_by!(user: users(:david))
    grant = HuddleGrant.issue!(session: users(:david).sessions.create!(user_agent: "Test"), membership: membership)
    grant.update_columns(last_seen_at: 31.seconds.ago)
    stream = Stream.create!(room: room, membership: membership, user: users(:david), quality: "1080p15")

    Huddle::Reconciler.new.reconcile_once

    assert_not_predicate stream.reload, :live?
  end
end
