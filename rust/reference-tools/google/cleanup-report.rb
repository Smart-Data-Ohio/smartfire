require 'json'
ActiveJob::Base.queue_adapter=:test
now=Time.utc(2026,3,2,16)
Time.define_singleton_method(:current) { now }
ENV['GOOGLE_CLIENT_ID']='test-client-id';ENV['GOOGLE_CLIENT_SECRET']='FAKE-report-secret'
rows=[]
[12345,nil].each do |account_id|
  calls=[];reports=[]
  http=Object.new
  http.define_singleton_method(:delete) do |path,*|
    calls << {method:'DELETE',path:}
    response=Net::HTTPTooManyRequests.new('1.1','429','fixture');response.instance_variable_set(:@read,true);response.body='{}';response
  end
  Net::HTTP.define_singleton_method(:start) { |host,*args,**options,&block|raise 'unrecorded host' unless host=='www.googleapis.com';block.call(http) }
  subscriber=Object.new
  subscriber.define_singleton_method(:report) do |error,handled:,severity:,context:,source:|
    reports << {error_class:error.class.name,handled:,severity:,context:context.transform_values { |v|v.is_a?(ActiveJob::Base) ? v.class.name : v },source:,tokens_absent:!error.inspect.include?('access-token') && !error.inspect.include?('refresh-token')}
  end
  Rails.error.subscribe(subscriber)
  snapshot={'access_token'=>'access-token','refresh_token'=>'refresh-token','access_token_expires_at'=>(now+10.days).iso8601}
  job=Calendar::DisconnectCleanupJob.new(['orphan-id'],snapshot,account_id)
  jobs=[]
  8.times do
    ActiveJob::Base.queue_adapter.enqueued_jobs.clear
    job.perform_now
    queued=ActiveJob::Base.queue_adapter.enqueued_jobs
    jobs << queued.map { |j|j[:job].name }
    job=Calendar::DisconnectCleanupJob.deserialize(queued.fetch(0)) unless queued.empty?
  end
  rows << {account_id:,calls:,jobs:,reports:}
  Rails.error.unsubscribe(subscriber)
end
puts JSON.pretty_generate({reference:'d7c7de92',rows:})
