# Remaining board automation contracts from Smartfire's pinned Rails source.
require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16)
ActiveJob::Base.queue_adapter = :test
Rails.logger = ActiveSupport::Logger.new(File::NULL)
root = ENV.fetch('PARITY_WORK')
JSON.parse(File.read(File.join(root, 'reference-tools/board_automations/dispatch-source-hashes.json'))).each do |path, hash|
  raise "Rails source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
conn = ActiveRecord::Base.connection
now = Time.current
board = 486777696
creator = 127326141
owner = 149087659
bot = 394959859
setup = ["DELETE FROM board_sla_nudges", "DELETE FROM board_sla_rules", "DELETE FROM board_stale_digests", "DELETE FROM activity_items", "UPDATE rooms SET type='Rooms::Board',creator_id=#{creator},deleted_at=NULL WHERE id=#{board}"]
base = {room_id: board, work_status: 'in_progress', nudge_after_minutes: '60', escalate_after_minutes: '240'}
models = []
[
  ['valid', {}], ['blocked', {work_status: 'blocked'}], ['planned', {work_status: 'planned'}],
  ['done', {work_status: 'done'}], ['unknown', {work_status: 'shipped'}], ['blank-status', {work_status: ''}], ['nil-status', {work_status: nil}],
  ['missing-room', {room_id: 0}], ['channel', {room_id: 654632876}], ['duplicate', {}, true], ['self-id-zero', {}, true],
  *[nil, '', ' ', '0', '-5', '43201', '60.0', '1e2', ' 60 ', '+60', '060', 'xyz'].map { |v| ["nudge-#{v.inspect}", {nudge_after_minutes: v}] },
  ['nil-escalation', {escalate_after_minutes: nil}], ['blank-escalation', {escalate_after_minutes: ''}],
  ['equal-escalation', {escalate_after_minutes: '60'}], ['earlier-escalation', {escalate_after_minutes: '30'}], ['max', {nudge_after_minutes: '43199', escalate_after_minutes: '43200'}]
].each do |name, overrides, duplicate|
  ActiveRecord::Base.transaction do
    setup.each { |sql| conn.execute(sql) }
    saved = BoardSlaRule.create!(base.merge(id: 0)) if duplicate
    rule = name == 'self-id-zero' ? saved : BoardSlaRule.new(base.merge(overrides))
    valid = rule.valid?
    models << {name:, input: base.merge(overrides), existing_id: name == 'self-id-zero' ? 0 : nil, duplicate: !!duplicate, valid:, errors: rule.errors.to_hash, full_messages: rule.errors.full_messages,
      nudge: rule.nudge_after_minutes, escalate: rule.escalate_after_minutes}
    raise ActiveRecord::Rollback
  end
end
thread_sql = ->(id, name, work_status, entered, work_owner=owner, room=board) do
  escaped = conn.quote(name)
  "INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,work_owner_id,work_status_changed_at,created_at,updated_at,last_activity_at) VALUES(#{id},#{room},#{creator},#{escaped},#{conn.quote(work_status)},#{work_owner || 'NULL'},#{conn.quote(entered)},'2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')"
end
rule_sql = "INSERT INTO board_sla_rules(room_id,work_status,nudge_after_minutes,escalate_after_minutes,created_at,updated_at) VALUES(#{board},'in_progress',60,240,'2026-03-02 16:00:00','2026-03-02 16:00:00')"
facts = -> do
  {claims: BoardSlaNudge.order(:id).pluck(:channel_thread_id,:work_status,:stage,:recipient_id,:status_entered_at).map { |r| r[4]=r[4].strftime('%Y-%m-%d %H:%M:%S.%6N'); r },
   items: ActivityItem.where(source_type: 'BoardSlaNudge').order(:id).pluck(:user_id,:event_type),
   pushes: ActiveJob::Base.queue_adapter.enqueued_jobs.select { |j| j[:job] == BoardAutomations::NudgePushJob }.map { |j| GlobalID::Locator.locate(j[:args][0]['_aj_globalid']).recipient_id }}
end
sla = []
%w[under nudge-boundary before-nudge escalation-boundary before-escalation unassigned creator-owner agent-owner inactive-owner revoked-owner bot-creator no-recipient deleted channel done unruled nil-entry untracked legacy-invalid-status repeat new-crossing later-escalation].each do |kind|
  ActiveRecord::Base.transaction do
    sql = setup + [rule_sql]
    entered = case kind
    when 'under' then '2026-03-02 15:30:00'
    when 'before-nudge' then '2026-03-02 15:00:00.000001'
    when 'nudge-boundary' then '2026-03-02 15:00:00'
    when 'before-escalation' then '2026-03-02 12:00:00.000001'
    when 'later-escalation' then '2026-03-02 12:01:00'
    when 'nil-entry' then nil
    else '2026-03-02 12:00:00'
    end
    status = {'done'=>'done','unruled'=>'planned','untracked'=>nil,'legacy-invalid-status'=>'mystery'}.fetch(kind, 'in_progress')
    who = {'unassigned'=>nil,'creator-owner'=>creator,'agent-owner'=>bot,'no-recipient'=>nil}.fetch(kind, owner)
    sql << thread_sql.call(970000001, 'Stale <&> post', status, entered, who)
    sql << "UPDATE users SET status=1 WHERE id=#{owner}" if kind == 'inactive-owner'
    sql << "DELETE FROM memberships WHERE room_id=#{board} AND user_id=#{owner}" if kind == 'revoked-owner'
    sql << "UPDATE rooms SET creator_id=#{bot} WHERE id=#{board}" if %w[bot-creator no-recipient].include?(kind)
    sql << "UPDATE rooms SET deleted_at='2026-03-02 16:00:00' WHERE id=#{board}" if kind == 'deleted'
    sql << "UPDATE rooms SET type='Rooms::Open' WHERE id=#{board}" if kind == 'channel'
    sql << "INSERT INTO board_sla_rules(room_id,work_status,nudge_after_minutes,escalate_after_minutes,created_at,updated_at) VALUES(#{board},'done',60,240,'2026-03-02 16:00:00','2026-03-02 16:00:00')" if kind == 'done'
    sql = sql.map {|q|q.sub("'in_progress'", "'mystery'")} if kind == 'legacy-invalid-status'
    sql.each { |s| conn.execute(s) }
    ActiveJob::Base.queue_adapter.enqueued_jobs.clear
    BoardAutomations::SlaDispatcher.dispatch_due!(now: now)
    runs = [{now: now.strftime('%Y-%m-%d %H:%M:%S.%6N'), setup: [], facts: facts.call}]
    extra = []
    later = now
    if kind == 'new-crossing'
      extra << "UPDATE channel_threads SET work_status_changed_at='2026-03-02 11:00:00' WHERE id=970000001"
    elsif kind == 'later-escalation'
      later += 60
    end
    if %w[repeat new-crossing later-escalation].include?(kind)
      extra.each { |s| conn.execute(s) }
      BoardAutomations::SlaDispatcher.dispatch_due!(now: later)
      runs << {now: later.strftime('%Y-%m-%d %H:%M:%S.%6N'), setup: extra, facts: facts.call}
    end
    sla << {name: kind, setup: sql, runs:}
    raise ActiveRecord::Rollback
  end
end
digests = []
%w[empty under boundary before-boundary quiet escaping many done nil-entry untracked deleted channel no-rules legacy-invalid-status repeat next-day age-minute age-hour age-day].each do |kind|
  ActiveRecord::Base.transaction do
    sql = setup + (kind == 'no-rules' ? [] : [rule_sql])
    entered = case kind
    when 'under' then '2026-03-02 15:30:00'
    when 'before-boundary' then '2026-03-02 15:00:00.000001'
    when 'boundary' then '2026-03-02 15:00:00'
    when 'age-minute' then '2026-03-02 15:59:00'
    when 'age-hour' then '2026-03-02 14:30:00'
    when 'age-day' then '2026-02-28 16:00:00'
    when 'nil-entry' then nil
    else '2026-03-02 14:00:00'
    end
    sql = setup + [rule_sql.sub('60,240', '1,240')] if kind == 'age-minute'
    status = {'done'=>'done','untracked'=>nil,'legacy-invalid-status'=>'mystery'}.fetch(kind, 'in_progress')
    count = kind == 'many' ? 23 : (kind == 'empty' ? 0 : 1)
    count.times do |i|
      sql << thread_sql.call(970000001+i, kind == 'escaping' ? '**Bold** <&> title' : "Old work #{i+1}", status, entered, i == 1 ? nil : owner)
    end
    sql << "UPDATE users SET name='*Kevin* <&>' WHERE id=#{owner}" if kind == 'escaping'
    sql << "UPDATE rooms SET deleted_at='2026-03-02 16:00:00' WHERE id=#{board}" if kind == 'deleted'
    sql << "UPDATE rooms SET type='Rooms::Open' WHERE id=#{board}" if kind == 'channel'
    sql << "UPDATE memberships SET unread_at=NULL WHERE room_id=#{board}"
    sql = sql.map {|q|q.sub("'in_progress'", "'mystery'")} if kind == 'legacy-invalid-status'
    sql.each { |s| conn.execute(s) }
    before = Message.count
    ActiveJob::Base.queue_adapter.enqueued_jobs.clear
    expected = -> {
      {digests: BoardStaleDigest.order(:id).map { |d| {room_id: d.room_id, on: d.digest_on.to_s, body: d.message.body.body.to_html, plain: d.message.plain_text_body, system_note: d.message.system_note, thread_id: d.message.thread_id, streaming: d.message.streaming} },
       message_delta: Message.count-before, items: ActivityItem.count, unread: Membership.where(room_id: board).where.not(unread_at: nil).count,
       jobs: ActiveJob::Base.queue_adapter.enqueued_jobs.map { |j| j[:job].name }}
    }
    BoardAutomations::DigestDispatcher.dispatch_due!(now: now)
    runs = [{now: now.strftime('%Y-%m-%d %H:%M:%S.%6N'), facts: expected.call}]
    if %w[repeat next-day].include?(kind)
      later = kind == 'next-day' ? now + 1.day : now
      BoardAutomations::DigestDispatcher.dispatch_due!(now: later)
      runs << {now: later.strftime('%Y-%m-%d %H:%M:%S.%6N'), facts: expected.call}
    end
    digests << {name: kind, setup: sql, runs:}
    raise ActiveRecord::Rollback
  end
end
query_counts = []
%w[sla digest].each do |kind|
  [10,100].each do |size|
    ActiveRecord::Base.transaction do
      setup.each { |sql| conn.execute(sql) }
      size.times do |i|
        room = 980000000+i
        conn.execute("INSERT INTO rooms(id,name,type,creator_id,created_at,updated_at) VALUES(#{room},'Sweep board','Rooms::Board',#{creator},'2026-03-02 16:00:00','2026-03-02 16:00:00')")
        [creator,owner].each { |user| conn.execute("INSERT INTO memberships(room_id,user_id,created_at,updated_at) VALUES(#{room},#{user},'2026-03-02 16:00:00','2026-03-02 16:00:00')") }
        conn.execute(rule_sql.sub(board.to_s,room.to_s))
        conn.execute(thread_sql.call(980001000+i,'Sweep post','in_progress','2026-03-02 14:00:00',owner,room))
      end
      2.times do |run|
        selects = []
        subscriber = ActiveSupport::Notifications.subscribe('sql.active_record') do |*args|
          event = args.last
          selects << event[:sql] if !event[:cached] && event[:name] != 'SCHEMA' && event[:sql].match?(/\A\s*SELECT/i)
        end
        begin
          if kind == 'sla'
            BoardAutomations::SlaDispatcher.dispatch_due!(now: now)
          else
            BoardAutomations::DigestDispatcher.dispatch_due!(now: now)
          end
        ensure
          ActiveSupport::Notifications.unsubscribe(subscriber)
        end
        query_counts << {kind:,size:,repeat:run==1,reads:selects.size}
      end
      raise ActiveRecord::Rollback
    end
  end
end
failures=[]
['post','attach'].each do |kind|
  ActiveRecord::Base.transaction do
    sql=setup+[rule_sql,thread_sql.call(970000001,'Failure post','in_progress','2026-03-02 14:00:00')]
    sql.each{|q|conn.execute(q)}
    trigger=kind=='post' ? "CREATE TRIGGER reject_digest BEFORE INSERT ON messages WHEN NEW.system_note=1 BEGIN SELECT RAISE(ABORT,'reject note'); END" : "CREATE TRIGGER reject_digest BEFORE UPDATE OF message_id ON board_stale_digests BEGIN SELECT RAISE(ABORT,'reject link'); END"
    conn.execute(trigger)
    before=Message.count
    BoardAutomations::DigestDispatcher.dispatch_due!(now:now)
    failures << {name:kind,setup:sql,trigger:,claims:BoardStaleDigest.count,message_delta:Message.count-before,attached:BoardStaleDigest.where.not(message_id:nil).count}
    raise ActiveRecord::Rollback
  end
end
cleanup=[]
2.times do |kind|
  ActiveRecord::Base.transaction do
    sql=setup+[rule_sql,thread_sql.call(970000001,'Cleanup post','in_progress','2026-03-02 13:00:00')]
    sql << "INSERT INTO board_tag_assignments(room_id,tag,assignee_id,created_by_id,created_at,updated_at) VALUES(#{board},'bug',#{owner},#{creator},'2026-03-02 16:00:00','2026-03-02 16:00:00')"
    sql.each {|q|conn.execute(q)}
    BoardAutomations::SlaDispatcher.dispatch_due!(now:now)
    BoardAutomations::DigestDispatcher.dispatch_due!(now:now)
    if kind==1
      post=ChannelThread.find(970000001)
      reply=post.post_message!(creator:User.find(creator),attributes:{markdown_source:'Which option?'})
      poll=Poll.create_for_message!(message:reply,labels:['A','B'])
      poll.cast_vote!(User.find(owner),[poll.poll_options.first.id])
      pending=ScheduledMessage.create!(user:User.find(creator),room:Room.find(board),thread:post,markdown_source:'Threaded nudge',send_at:now+3600)
      dropped=ScheduledMessage.create!(user:User.find(creator),room:Room.find(board),markdown_source:'Root post',send_at:now+3600)
      dropped.drop!(reason:'test')
    end
    Room.find(board).begin_destroy!
    Room::DestroyJob.perform_now(board)
    tables=%w[rooms messages channel_threads board_tag_assignments board_sla_rules board_sla_nudges board_stale_digests scheduled_messages]
    counts=tables.to_h{|table|[table,conn.select_value("SELECT COUNT(*) FROM #{table} WHERE #{table=='rooms' ? 'id' : 'room_id'}=#{board}").to_i]}
    counts['sla_items']=ActivityItem.where(event_type:'work_sla').count
    counts['polls']=Poll.where(id:poll.id).count if kind==1
    counts['poll_options']=PollOption.where(poll_id:poll.id).count if kind==1
    counts['poll_votes']=PollVote.where(poll_id:poll.id).count if kind==1
    counts['scheduled_items']=ActivityItem.where(source_type:'ScheduledMessage',source_id:[pending.id,dropped.id]).count if kind==1
    cleanup << {with_scheduled:kind==1,setup:sql,counts:}
    raise ActiveRecord::Rollback
  end
end
cadence = Periodic::Runner.new.instance_variable_get(:@tasks).select { |t| t.name.start_with?('board ') }.map { |t| {name: t.name, seconds: t.interval.to_i} }
puts JSON.pretty_generate(reference: 'd7c7de92 plus approved board drift', now: now.strftime('%Y-%m-%d %H:%M:%S.%6N'), models:, sla:, digests:, failures:, cleanup:, cadence:, query_counts:)
warn "Rails board automation oracle: #{models.size} rule cases; #{sla.size} SLA cases; #{digests.size} digest cases; #{failures.size} failure cases; #{cleanup.size} cleanup cases; #{cadence.size} cadences; #{query_counts.size} query probes; 0 masks"
