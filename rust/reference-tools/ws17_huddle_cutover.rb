# The two formerly owner-held mixed PushGating scenarios, using
# actual grant issuance, invitation jobs, policy and overdue resolution.
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
ActiveRecord::Base.logger = nil
ActiveJob::Base.queue_adapter = :test
ENV['LIVEKIT_API_SECRET'] = 'fixture-cutover-huddle-secret'
conn = ActiveRecord::Base.connection
def insert_sql(record)
  c = ActiveRecord::Base.connection
  a = record.attributes_for_database
  "INSERT OR REPLACE INTO #{c.quote_table_name(record.class.table_name)} (#{a.keys.map { |k| c.quote_column_name(k) }.join(',')}) VALUES (#{a.values.map { |v| c.quote(v) }.join(',')});"
end
rows = []
travel_to(Time.utc(2026, 9, 23, 12)) do
  %w[dnd_allowed group_quiet].each do |name|
    ActiveRecord::Base.transaction(joinable: false) do
      users = %i[david jason kevin jz].map { |key| User.find(ActiveRecord::FixtureSet.identify(key)) }
      caller, recipient, quiet, reachable = users
      users.each { |u| u.update!(inbox_preferences: {}, dnd_enabled: false, quiet_hours_enabled: false, presence_setting: 'auto', time_zone: 'UTC') }
      DndAllowedUser.delete_all
      ActivityItem.where(source_type: 'HuddleGrant').delete_all
      HuddleGrant.delete_all
      Rooms::Direct.update_all(direct_member_key: nil)
      group = name == 'group_quiet'
      if group
        recipient.update!(dnd_enabled: true)
        quiet.update!(quiet_hours_enabled: true, quiet_hours_start: '09:00', quiet_hours_end: '17:00')
      end
      room = group ? Rooms::Direct.create_for({creator: caller}, users:) : Room.find(ActiveRecord::FixtureSet.identify(:david_and_jason))
      room.memberships.update_all(involvement: 'everything', connections: 0, connected_at: nil, created_at: Time.current, updated_at: Time.current)
      session = Session.find(ActiveRecord::FixtureSet.identify(:david_safari))
      setup = ["DELETE FROM activity_items WHERE source_type='HuddleGrant'; DELETE FROM huddle_grants; DELETE FROM dnd_allowed_users; DELETE FROM push_subscriptions;"]
      setup += [room, *room.memberships, *users, session, *Push::Subscription.where(user_id: users.map(&:id))].map { |r| insert_sql(r) }
      grant = HuddleGrant.issue!(session:, membership: room.memberships.find_by!(user: caller))
      recipients = group ? [recipient, quiet, reachable] : [recipient]
      items = recipients.map { |u| ActivityItem.find_by!(user: u, source: grant) }
      state = -> { items.map { |i| [i.reload.user_id, i.event_type, i.unread?] } }
      initial = state.call
      deliveries = []
      pool = Object.new
      pool.define_singleton_method(:queue) do |payload, subscriptions|
        deliveries << {payload:, users: subscriptions.order(:id).pluck(:user_id)}
      end
      original = Rails.configuration.x.web_push_pool
      Rails.configuration.x.web_push_pool = pool
      begin
        recipient.update!(dnd_enabled: true) unless group
        items.each { |i| Huddle::PushInvitationJob.perform_now(i.id) }
        first_deliveries = deliveries.deep_dup
        if name == 'dnd_allowed'
          DndAllowedUser.create!(user: recipient, allowed_user: caller)
          Huddle::PushInvitationJob.perform_now(items.first.id)
        end
        final = nil
        if group
          items.each { |i| i.update_column(:created_at, 46.seconds.ago) }
          Huddle::InvitationResolver.resolve_overdue!
          final = state.call
        end
        rows << {name:, setup_sql: setup.join("\n"), room: room.id, session: session.id,
          caller: caller.id, recipients: recipients.map(&:id), initial:, first_deliveries:, deliveries:, final:}
      ensure
        Rails.configuration.x.web_push_pool = original
      end
      raise ActiveRecord::Rollback
    end
  end
end
puts JSON.pretty_generate(reference: 'd7c7de92', now: '2026-09-23T12:00:00Z', rows:)
warn "Rails huddle cutover: #{rows.length} source scenarios; 0 failures"
