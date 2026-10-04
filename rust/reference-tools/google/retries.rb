# Actual Calendar job rescue handlers, serialized between each execution; no HTTP.
require 'json'
ActiveJob::Base.queue_adapter=:test
now=Time.utc(2026,3,2,16)
Time.define_singleton_method(:current) { now }
Kernel.define_singleton_method(:rand) { 0.0 }
errors={
  'google'=>-> { Google::Client::Unavailable.new('recorded transport outage') },
  'inherited'=>-> { ActiveRecord::StatementTimeout.new('recorded database timeout') },
  'permanent'=>-> { RuntimeError.new('recorded permanent error') }
}
sequence=nil
Calendar::InboundSync.define_singleton_method(:sync) { |id| raise errors.fetch(sequence).call }
Calendar::MeetLink.define_singleton_method(:provision!) { |event| raise errors.fetch(sequence).call }
event=Event.new(id:123)
Event.define_singleton_method(:find_by) { |**args| event }
out={reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],cases:[]}
[Calendar::InboundSyncJob,Calendar::MeetLinkJob].each do |klass|
  {
    'mixed_google_exhaustion'=>%w[inherited inherited inherited inherited google google google google google google google google],
    'mixed_inherited_exhaustion'=>%w[google google google google google google google inherited inherited inherited inherited inherited],
    'permanent'=>%w[permanent]
  }.each do |name,steps|
    job=klass.new(123);rows=[]
    steps.each do |kind|
      sequence=kind
      ActiveJob::Base.queue_adapter.enqueued_jobs.clear
      raised=nil
      begin
        job.perform_now
      rescue => e
        raised=e.class.name
      end
      queued=ActiveJob::Base.queue_adapter.enqueued_jobs.last
      rows << {kind:,executions:job.executions,counts:job.exception_executions.dup,retry:!!queued,wait:queued ? queued[:at]-now.to_f : nil,raised:}
      break unless queued
      job=ActiveJob::Base.deserialize(job.serialize)
    end
    out[:cases] << {class:klass.name,name:,steps:rows}
  end
end
puts JSON.pretty_generate(out)
