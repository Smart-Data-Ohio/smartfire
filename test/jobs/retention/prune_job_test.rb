require "test_helper"

class Retention::PruneJobTest < ActiveJob::TestCase
  test "prunes old agent events and keeps recent ones" do
    agent = agents(:bender_agent)
    old_event = travel_to(91.days.ago) do
      agent.agent_events.create!(event_type: "mention", outcome: "acknowledged")
    end
    new_event = travel_to(89.days.ago) do
      agent.agent_events.create!(event_type: "mention", outcome: "acknowledged")
    end

    Retention::PruneJob.perform_now

    assert_empty AgentEvent.where(id: old_event.id)
    assert AgentEvent.exists?(new_event.id)
  end

  test "prunes seen activity items but never unread ones" do
    message = messages(:first)
    old_read = ActivityItem.create!(user: users(:jason), source: message, event_type: "mention")
    old_read.update_columns(read_at: 181.days.ago, updated_at: 181.days.ago)
    old_handled = ActivityItem.create!(user: users(:kevin), source: message, event_type: "reply")
    old_handled.update_columns(read_at: 181.days.ago, handled_at: 181.days.ago, updated_at: 181.days.ago)
    old_unread = ActivityItem.create!(user: users(:david), source: message, event_type: "mention")
    old_unread.update_columns(created_at: 200.days.ago, updated_at: 200.days.ago)
    new_read = ActivityItem.create!(user: users(:jz), source: message, event_type: "mention")
    new_read.update_columns(read_at: 10.days.ago, updated_at: 10.days.ago)

    Retention::PruneJob.perform_now

    assert_empty ActivityItem.where(id: [ old_read.id, old_handled.id ])
    assert ActivityItem.exists?(old_unread.id)
    assert ActivityItem.exists?(new_read.id)
  end

  test "prunes old webhook deliveries and keeps recent ones" do
    old_delivery = travel_to(15.days.ago) do
      Github::WebhookDelivery.create!(delivery_guid: "old-guid", event: "pull_request")
    end
    new_delivery = travel_to(13.days.ago) do
      Github::WebhookDelivery.create!(delivery_guid: "new-guid", event: "pull_request")
    end

    Retention::PruneJob.perform_now

    assert_empty Github::WebhookDelivery.where(id: old_delivery.id)
    assert Github::WebhookDelivery.exists?(new_delivery.id)
  end

  test "prunes completed cleanups but never pending ones" do
    old_completed = HuddleCleanup.create!(operation: :delete_room, room_name: "old-room", completed_at: 8.days.ago)
    new_completed = HuddleCleanup.create!(operation: :delete_room, room_name: "new-room", completed_at: 6.days.ago)
    old_pending = travel_to(30.days.ago) do
      HuddleCleanup.create!(operation: :delete_room, room_name: "pending-room")
    end

    Retention::PruneJob.perform_now

    assert_empty HuddleCleanup.where(id: old_completed.id)
    assert HuddleCleanup.exists?(new_completed.id)
    assert HuddleCleanup.exists?(old_pending.id)
  end

  test "prunes revoked grants with their inbox items and keeps the rest" do
    room = rooms(:watercooler)
    membership = memberships(:david_watercooler)

    old_grant = HuddleGrant.create!(identity: "old-grant", room_name: "room", room:,
      session: sessions(:david_safari), user: users(:david), membership:,
      revoked_at: 31.days.ago)
    old_item = ActivityItem.create!(user: users(:david), source: old_grant, event_type: "huddle_missed")
    old_cleanup = HuddleCleanup.create!(operation: :remove_participant, room_name: "room",
      identity: "old-grant", huddle_grant: old_grant)

    new_grant = HuddleGrant.create!(identity: "new-grant", room_name: "room", room:,
      session: sessions(:david_safari), user: users(:david), membership:,
      revoked_at: 29.days.ago)
    active_grant = travel_to(60.days.ago) do
      HuddleGrant.create!(identity: "active-grant", room_name: "room", room:,
        session: sessions(:david_safari), user: users(:david), membership:)
    end

    Retention::PruneJob.perform_now

    assert_empty HuddleGrant.where(id: old_grant.id)
    assert_empty ActivityItem.where(id: old_item.id)
    assert HuddleGrant.exists?(new_grant.id)
    assert HuddleGrant.exists?(active_grant.id)
    # The cleanup survives the unlink with its own room copies.
    assert_nil old_cleanup.reload.huddle_grant_id
    assert HuddleCleanup.exists?(old_cleanup.id)
  end

  test "prunes revoked grants with one cleanup unlink per batch" do
    room = rooms(:watercooler)
    membership = memberships(:david_watercooler)

    3.times do |index|
      grant = HuddleGrant.create!(identity: "batch-grant-#{index}", room_name: "room", room:,
        session: sessions(:david_safari), user: users(:david), membership:,
        revoked_at: 31.days.ago)
      HuddleCleanup.create!(operation: :remove_participant, room_name: "room",
        identity: grant.identity, huddle_grant: grant)
    end

    updates = 0
    ActiveSupport::Notifications.subscribed(
      ->(*, payload) { updates += 1 if payload[:sql].start_with?("UPDATE") && payload[:sql].include?("huddle_cleanups") },
      "sql.active_record"
    ) do
      Retention::PruneJob.perform_now
    end

    assert_equal 1, updates
    assert_empty HuddleGrant.where("identity LIKE 'batch-grant-%'")
    assert_empty HuddleCleanup.where("identity LIKE 'batch-grant-%'").where.not(huddle_grant_id: nil)
  end

  test "prunes fizzy card caches older than a day and keeps recent ones" do
    card = Fizzy::Card.for_reference(account_id: "897362094", number: 579)
    old_cache = Fizzy::CardCache.for_viewer(card: card, user: users(:david))
    old_cache.update_columns(updated_at: 2.days.ago)
    new_cache = Fizzy::CardCache.for_viewer(card: card, user: users(:jz))

    Retention::PruneJob.perform_now

    assert_empty Fizzy::CardCache.where(id: old_cache.id)
    assert Fizzy::CardCache.exists?(new_cache.id)
  end

  test "re-enqueues destroys for rooms stuck as deleted" do
    stuck = Rooms::Closed.create_for({ name: "Stuck", creator: users(:david) }, users: [ users(:david) ])
    stuck.begin_destroy!
    stuck.update_columns(deleted_at: 2.hours.ago)

    fresh = Rooms::Closed.create_for({ name: "Fresh", creator: users(:david) }, users: [ users(:david) ])
    fresh.begin_destroy!

    assert_enqueued_with(job: Room::DestroyJob, args: [ stuck.id ]) do
      Retention::PruneJob.perform_now
    end

    destroy_args = enqueued_jobs.select { |job| job[:job] == Room::DestroyJob }.map { |job| job[:args] }
    assert_equal [ [ stuck.id ] ], destroy_args
  end
end
