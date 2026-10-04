# WorkThreadLink validations, normalization and persistence from the pinned Rails model.
require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
hashes=JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/work/source-hashes.json')))
%w[app/models/work_thread_link.rb app/models/channel_thread.rb].each { |file|raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest==hashes.fetch(file) }
now='2026-03-02 16:00:00'
setup=[
 "INSERT INTO channel_threads(id,room_id,creator_id,name,work_status,created_at,updated_at,last_activity_at) VALUES(900083001,486777696,127326141,'Linked work','planned','#{now}','#{now}','#{now}'),(900083002,486777696,127326141,'Other work','planned','#{now}','#{now}','#{now}')",
 "INSERT INTO events(id,room_id,organizer_id,title,starts_at,time_zone,created_at,updated_at) VALUES(900083003,486777696,127326141,'Same room','2026-03-03 16:00:00','UTC','#{now}','#{now}'),(900083004,654632876,127326141,'Other room','2026-03-03 16:00:00','UTC','#{now}','#{now}')",
 "INSERT INTO github_pull_requests(id,owner,repo,number,created_at,updated_at) VALUES(900083005,'rails','rails',90999999,'#{now}','#{now}')"
]
base={channel_thread_id:900083001,created_by_id:127326141}
pr={kind:'pull_request',github_pull_request_id:900083005}
event={kind:'event',event_id:900083003}
drive={kind:'drive_file',url:'https://drive.google.com/file/d/abcdefghij'}
cases=[
 ['pr',pr,nil],['pr-missing',{kind:'pull_request'},nil],['pr-event',pr.merge(event_id:900083003),nil],['pr-url',pr.merge(url:'url'),nil],['pr-title',pr.merge(title:'title'),nil],
 ['event',event,nil],['event-missing',{kind:'event'},nil],['event-pr',event.merge(github_pull_request_id:900083005),nil],['event-url',event.merge(url:'url'),nil],['event-title',event.merge(title:'title'),nil],['event-other-room',event.merge(event_id:900083004),nil],
 ['drive',drive.merge(title:'Plan'),nil],['drive-missing',{kind:'drive_file'},nil],['drive-pr',drive.merge(github_pull_request_id:900083005),nil],['drive-event',drive.merge(event_id:900083003),nil],['drive-long',drive.merge(title:'é'*300),nil],['drive-plain-model-url',drive.merge(url:'not a recognised Drive URL'),nil],
 ['kind-missing',{},nil],['kind-empty',{kind:''},nil],['missing-thread',drive.merge(channel_thread_id:0),nil],['missing-creator',drive.merge(created_by_id:0),nil],
 ['pr-duplicate',pr,pr],['event-duplicate',event,event],['drive-duplicate',drive,drive],['other-thread',drive.merge(channel_thread_id:900083002),drive],['nil-event-duplicate',{kind:'event'},drive],['nil-pr-duplicate',{kind:'pull_request'},drive],['nil-url-duplicate',{kind:'drive_file'},pr],
 ['pr-blank-columns',pr.merge(url:' ',title:' '),nil]
]
rows=cases.map do |name,input,duplicate|
 row=nil
 ActiveRecord::Base.transaction do
  setup.each { |sql|ActiveRecord::Base.connection.execute(sql) }
  WorkThreadLink.create!(base.merge(duplicate)) if duplicate
  record=WorkThreadLink.new(base.merge(input));valid=record.valid?
  before=ActivityItem.count;thread_time=ChannelThread.find(record.channel_thread_id).updated_at if record.channel_thread_id!=0
  record.save! if valid
  row={name:,input:base.merge(input),duplicate:duplicate && base.merge(duplicate),valid:,errors:record.errors.to_hash,full_messages:record.errors.full_messages,title:record.title,url:record.url,
   stored:valid && record.persisted?,activity_delta:ActivityItem.count-before,thread_touched:thread_time && ChannelThread.find(record.channel_thread_id).updated_at!=thread_time}
  raise ActiveRecord::Rollback
 end
 row
end
puts JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA"),setup:,rows:)
warn "Rails work link model oracle: #{rows.size} validation/persistence cases; 0 masks"
