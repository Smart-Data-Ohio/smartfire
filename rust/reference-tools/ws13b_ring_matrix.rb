require "json"
require "digest"
require "active_support/testing/time_helpers"
require "active_job/test_helper"

# Observe real after-commit Cable callbacks and execute actual ActiveJob classes.
# The only replaced boundary is the outbound pool (no network transport in probes).
class WS13bRingMatrix
  include ActiveSupport::Testing::TimeHelpers
  def specs
    families = []
    [0, 1, 120, 121, 181].each do |delay|
      families << ["pending_initial_reissue_#{delay}", [step("issue"), step("issue", delay), step("drain")]]
      families << ["pending_retry_reissue_#{delay}", [step("issue"), step("drain"), step("issue", 181), step("issue", delay), step("drain")]]
    end
    families << ["two_pending_retries", [step("issue"), step("issue", 181), step("issue", 181), step("drain")]]
    families << ["sliding_dedupe", [step("issue"), step("issue", 0), step("issue", 1), step("issue", 120), step("issue", 1), step("drain")]]
    [0, 1, 181].each { |delay| families << ["delayed_initial_#{delay}", [step("issue"), step("drain", delay)]] }
    %w[quiet_revoke live_revoke end remove_recipient remove_caller sign_out].each do |action|
      [false, true].each do |delivered|
        steps = [step("issue")]
        steps << step("drain") if delivered
        steps += [step(action, 1), step("drain")]
        families << ["#{action}_#{delivered ? 'delivered' : 'pending'}", steps]
      end
    end
    %w[live_revoke end].each do |action|
      [0, 1, 181].each { |delay| families << ["#{action}_regrant_#{delay}", [step("issue"), step(action), step("issue", delay), step("drain")]] }
      families << ["#{action}_regrant_dedupe", [step("issue"), step(action), step("issue", 181), step("issue", 1), step("drain")]]
    end
    [0, 1, 181].each { |delay| families << ["new_session_#{delay}", [step("issue"), step("new_session", delay), step("drain")]] }
    families << ["group_continues", [step("issue"), step("group_continues", 1), step("drain")]]
    cases = families.flat_map do |name, steps|
      [false, true].product([false, true]).map do |banner, newest|
        {name: "#{name}/#{banner ? 'banner' : 'item'}/#{newest ? 'newest' : 'oldest'}", banner: banner, newest_first: newest, steps: steps}
      end
    end
    %w[read handled missed unread_cycle].each do |mutation|
      [0, 1, 181].product([false, true]).each do |delay, newest|
        steps = [step("issue"), step("drain"), step(mutation, mutation == "missed" ? 46 : 1), step("issue", delay), step("drain")]
        cases << {name: "#{mutation}_retry_#{delay}/item/#{newest ? 'newest' : 'oldest'}", banner: false, newest_first: newest, steps: steps}
      end
    end
    cases
  end

  def step(action, seconds = 0) = {action: action, seconds: seconds}


  SEEDS = JSON.parse(ENV.fetch("WS13B_SEEDS", "[388013012,3620200082]")).freeze
  ACTIONS = %w[issue issue banner inbox retry dismiss read handled end quiet_revoke regrant rejoin drain drain].freeze
  DELAYS = [0, 1, 20, 21, 45, 46, 60, 61, 119, 120, 121, 180, 181, 600, 601].freeze

  def random_specs(count)
    SEEDS.each_with_index.flat_map do |seed,seed_index|
      rng = Random.new(seed)
      Array.new(count / SEEDS.size + (seed_index < count % SEEDS.size ? 1 : 0)) do |index|
        steps = [step("issue")]
        rng.rand(12..28).times do
          action = ACTIONS.sample(random: rng)
          seconds = action == "retry" ? [121, 181, 601].sample(random: rng) : DELAYS.sample(random: rng)
          operation = step(action, seconds)
          operation[:order] = %w[oldest newest shuffled].sample(random: rng) if action == "drain"
          operation[:limit] = rng.rand(1..4) if action == "drain" && rng.rand(2).zero?
          steps << operation
        end
        steps << step("drain").merge(order: "oldest")
        {name: "random/#{seed}/#{index}", seed: seed, banner: rng.rand(2).zero?, newest_first: false, steps: steps}
      end
    end
  end

  def regressions
    families = {
      "review_r4_cross_form" => [step("issue"), step("toggle_preferences",181), step("issue"), step("drain")],
      "review_r4_dismissed_cross_form" => [step("issue"),step("inbox",181),step("issue"),step("dismiss",1),step("drain")],
      "review_r4_handled_end" => [step("issue"),step("drain"),step("handled",1),step("end",1),step("drain")],
      "review_r2_delayed_handled_retry" => [step("issue"),step("drain"),step("handled",1),step("issue",181),step("drain")]
    }
    cases = families.flat_map do |name, steps|
      [false,true].product([false,true]).map { |banner,newest| {name: "#{name}/#{banner}/#{newest}",banner: banner,newest_first:newest,steps: steps} }
    end
    cases += [
      {name:"review_r5/banner_dismiss_history",banner:false,newest_first:false,steps:[step("issue"),step("banner",121),step("issue"),step("dismiss"),step("drain")]},
      {name:"review_r5/item_dismiss_patch",banner:false,newest_first:false,steps:[step("issue"),step("dismiss"),step("drain")]},
      {name:"review_r5/inbox_read_behind_banner",banner:false,newest_first:false,steps:[step("issue"),step("banner",121),step("issue"),step("read"),step("drain")]}
    ]
    cases + JSON.parse(File.read(File.join(__dir__,"ws13b_shrunk_sequences.json")),symbolize_names: true)
  end

  def setup(spec)
    connection = ActiveRecord::Base.connection
    unless @fixture_database
      ActiveRecord::Schema.verbose = false
      load Rails.root.join("db/schema.rb")
      ActiveRecord::FixtureSet.reset_cache
      ActiveRecord::FixtureSet.create_fixtures(Rails.root.join("test/fixtures"), %w[accounts users rooms memberships push/subscriptions])
      @database_path = ActiveRecord::Base.connection_db_config.database
      connection.execute("PRAGMA wal_checkpoint(TRUNCATE)")
      ActiveRecord::Base.connection_pool.disconnect!
      @fixture_database = File.binread(@database_path)
    end
    # Each sequence starts with a byte-identical copy of the real fixture DB.
    # Disconnect before replacing our private reference runner database, so no
    # statement, WAL or AUTOINCREMENT state leaks from the previous sequence.
    ActiveRecord::Base.connection_pool.disconnect!
    ["-wal", "-shm"].each { |suffix| File.delete(@database_path + suffix) if File.exist?(@database_path + suffix) }
    File.binwrite(@database_path, @fixture_database)
    ActiveRecord::Base.clear_query_caches_for_current_thread
    ActiveRecord::Base.connection.clear_query_cache
    Current.reset
    Rails.cache.clear
    travel_to Time.utc(2026, 1, 1, 12)
    ENV["LIVEKIT_API_SECRET"] = "ws13b-review-fixture-value"
    ENV.delete("LIVEKIT_URL"); ENV.delete("LIVEKIT_INTERNAL_URL")
    @caller, @recipient = %w[david jason].map { |key| User.find(ActiveRecord::FixtureSet.identify(key)) }
    spec[:recipient_id] = @recipient.id
    @room = Room.find(ActiveRecord::FixtureSet.identify("david_and_jason"))
    spec[:room_id] = @room.id
    spec[:client_ids] = [@caller.id,@recipient.id,ActiveRecord::FixtureSet.identify("kevin")].sort
    @member = @room.memberships.find_by!(user: @caller)
    @session = @caller.sessions.create!(token: "ws13b-matrix-session")
    @recipient.update!(inbox_preferences: {huddle_invitations: false}) if spec[:banner]
    @recipient_session = nil
    @grant = nil
    @frames, @pushes = [], []
    @adapter = ActiveJob::QueueAdapters::TestAdapter.new
    ActiveJob::Base.queue_adapter = @adapter
    @rng = Random.new(spec[:seed] || 172)
    probe = self
    ActionCable.server.define_singleton_method(:broadcast) { |stream,payload| probe.observe(stream,payload) }
    Rails.configuration.x.web_push_pool.define_singleton_method(:queue) { |payload,subscriptions| probe.push(payload,subscriptions) }
  end

  def observe(stream,payload)
    return unless (stream.match?(/\Auser_\d+_activity\z/) && payload[:huddleInvitation]) ||
      (stream.match?(/\Auser_\d+_huddle_notices\z/) && payload[:huddleJoinNotice])
    @frames << {stream: stream, payload: JSON.parse(JSON.generate(payload))}
  end
  def push(payload,subscriptions)
    # Pool handoffs, including empty scopes, are observed rather than predicted.
    @pushes << {payload: payload, subscription_ids: subscriptions.order(:id).pluck(:id)}
  end
  def issue = @grant = HuddleGrant.issue!(session: @session, membership: @member)
  def item = ActivityItem.where(user: @recipient,source_type: "HuddleGrant").order(:id).last

  # Inventory the entire adapter queue, including unexpected classes.
  def pending_jobs = @adapter.enqueued_jobs
  def queue_inventory
    pending_jobs.map { |row| {class:row[:job].name,arguments:row.fetch("arguments"),scheduled_at:row["scheduled_at"]} }
  end
  def ui_exchange(message)
    raise "Run with ws13b_differential.py: observed UI requests are required" unless ENV["WS13B_OBSERVED_UI"] == "1"
    puts JSON.generate(message);STDOUT.flush
    JSON.parse(STDIN.readline,symbolize_names:true)
  end

  def drain(spec,operation)
    jobs = pending_jobs
    originals = jobs.dup
    jobs.reverse! if operation[:order] == "newest" || (!operation[:order] && spec[:newest_first])
    jobs.shuffle!(random: @rng) if operation[:order] == "shuffled"
    jobs = jobs.first(operation[:limit]) if operation[:limit]
    operation[:job_order] = jobs.map { |row| originals.index(row) }
    jobs.each do |row|
      @adapter.enqueued_jobs.delete(row)
      ActiveJob::Base.execute(row.except(:job,:args,:queue,:priority,:at))
    end
    raise "complete drain left pending jobs" if !operation[:limit] && pending_jobs.any?
  end

  def apply(action)
    case action
    when "issue", "retry", "regrant" then issue
    when "banner", "inbox", "toggle_preferences"
      enabled = action == "inbox" || (action == "toggle_preferences" && !@recipient.reload.inbox_preferences.huddle_invitations)
      @recipient.update!(inbox_preferences: {huddle_invitations: enabled})
    when "new_session"
      @session = @caller.sessions.create!(token: "ws13b-matrix-second-session")
      issue
    when "quiet_revoke" then @grant&.reload&.revoke!(create_cleanup: false)
    when "live_revoke" then @grant.record_seen!; @grant.revoke!(create_cleanup: false)
    when "end" then @grant.record_seen!; @grant.mark_out_of_call!
    when "remove_recipient" then @room.memberships.find_by!(user: @recipient).destroy!
    when "remove_caller" then @grant.record_seen!; @member.destroy!
    when "sign_out" then @grant.record_seen!; @session.destroy!
    when "group_continues"
      other = User.find(ActiveRecord::FixtureSet.identify("kevin"))
      member = @room.memberships.create!(user: other, involvement: "everything")
      live = HuddleGrant.issue!(session: other.sessions.create!(token: "ws13b-matrix-group-session"),membership: member)
      live.record_seen!; @grant.record_seen!; @grant.revoke!(create_cleanup: false)
    when "rejoin"
      member = @room.memberships.find_by!(user: @recipient)
      @recipient_session ||= @recipient.sessions.create!(token: "ws13b-recipient-session")
      HuddleGrant.issue!(session: @recipient_session,membership: member).record_seen!
    when "read" then item&.mark_read!
    when "dismiss" then nil # The real displayed controller supplies its PATCH, if any.
    when "handled" then item&.mark_handled!
    when "missed" then Huddle::InvitationResolver.resolve_overdue!(user: @recipient)
    when "unread_cycle" then item&.mark_handled!; item&.mark_unread!
    else raise "unknown action #{action}"
    end
  end
  def snapshot
    ActivityItem.where(user: @recipient,source_type: "HuddleGrant").order(:id).map { |row|
      {id:row.id,source_id:row.source_id,event_type:row.event_type,state:row.state,created_at:row.created_at.iso8601(6)} }
  end
  def run(definitions_override = nil)
    count = Integer(ARGV.fetch(0,"512"))
    definitions = definitions_override || (specs + regressions + random_specs(count))
    definitions = JSON.parse(ENV["WS13B_SPEC_JSON"],symbolize_names: true) if ENV["WS13B_SPEC_JSON"]
    cases = definitions.map { |spec| Marshal.load(Marshal.dump(spec)) }.each_with_index.map do |spec,index|
      setup(spec)
      ui_exchange(ui_start:spec)
      phases = spec[:steps].map do |operation|
        travel operation[:seconds]
        requests = ui_exchange(ui_before:operation).fetch(:requests)
        operation[:ui_requests] = requests
        requests.each do |request|
          match = %r{\A/activity/(\d+)/read\?state=read\z}.match(request.fetch(:path))
          raise "unexpected UI request" unless request[:user_id] == @recipient.id && request[:method] == "PATCH" && match
          # ActivityItemsController#read (activity_items_controller.rb:52-60).
          ActivityItem.accessible_to(@recipient).find(match[1]).mark_read!
        end
        operation[:action] == "drain" ? drain(spec,operation) : apply(operation[:action])
        phase = {frames:@frames.shift(@frames.size),pushes:@pushes.shift(@pushes.size),items:snapshot,queue:queue_inventory,requests:requests}
        phase[:banners] = ui_exchange(ui_after:phase).fetch(:banners)
        phase
      end
      warn "observed Rails #{index+1}/#{definitions.size}" if (index+1) % 100 == 0
      {spec:spec,phases:phases}
    end
    files = %w[app/models/huddle_grant.rb app/models/activity_item.rb app/models/huddle/ring_policy.rb app/models/huddle/invitation_resolver.rb app/jobs/huddle/push_invitation_job.rb app/models/huddle/invitation_pusher.rb app/jobs/huddle/join_notice_job.rb app/models/huddle/join_notifier.rb app/models/huddle/join_pusher.rb app/javascript/controllers/huddle_invitation_controller.js app/controllers/activity_items_controller.rb]
    puts JSON.generate(reference_pin:"d7c7de92",source_sha256: files.to_h { |file| [file,Digest::SHA256.file(Rails.root.join(file)).hexdigest] },seeds:SEEDS,random_count:count,cases:cases)
  ensure
    travel_back
  end
end
WS13bRingMatrix.new.run
