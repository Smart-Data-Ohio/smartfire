# The three Drive delivery contracts through real polling and both Rails job consumers.
require 'json'
require 'webmock'
require 'fileutils'
require 'action_dispatch/testing/integration'
Rails.logger=ActiveSupport::Logger.new($stderr)
ActiveJob::Base.queue_adapter=:test
Rails.application.routes.default_url_options.merge!(host:'campfire.test',protocol:'http')
ActionController::Base.allow_forgery_protection=false
now=Time.utc(2026,3,2,16);Time.define_singleton_method(:current) {now}
rows=[]
%w[poll_files poll_empty webhook_files].each do |name|
  database=ActiveRecord::Base.connection_db_config.database
  ActiveRecord::Base.connection.execute('PRAGMA wal_checkpoint(TRUNCATE)');ActiveRecord::Base.connection_pool.disconnect!
  snapshot="#{database}.drive-delivery";FileUtils.cp(database,snapshot)
  begin
    Rails.application.executor.run!(reset:true)
    AgentEvent.delete_all;AgentGrant.delete_all;ActiveJob::Base.queue_adapter.enqueued_jobs.clear
    agent=Agent.find(773018776);room=Room.find(486777696);room.memberships.find_or_create_by!(user:agent.user)
    message=room.messages.create!(creator:User.find(127326141),markdown_source:'Hey @[Bender Bot], see these',client_message_id:"drive-#{name}")
    ids=name=='poll_empty' ? [] : name=='poll_files' ? %w[1AbcDefGhIjKlMnOpQrSt 2BcdEfgHiJkLmNoPqRsTu] : %w[1AbcDefGhIjKlMnOpQrSt]
    ids.each { |file_id|message.drive_attachments.create!(file_id:) }
    result=nil;http_observation=nil
    if name.start_with?('poll')
      client=ActionDispatch::Integration::Session.new(Rails.application);client.host! 'campfire.test'
      client.get('/agents/events?envelope=1',headers:{'Authorization'=>['Bearer','bender-test-secret-1234'].join(' ')})
      raise "poll status #{client.response.status}" unless client.response.status==200
      row=client.response.parsed_body['events'].find { |entry|entry.dig('message','id')==message.id }
      raise 'missing message event' unless row
      result={attachments:row.dig('message','drive_attachments')}
      http_observation={status:client.response.status,cache_control:client.response.headers['Cache-Control'],envelope:client.response.parsed_body.keys.sort,next_since_header_matches:client.response.headers['X-Smartfire-Next-Since']==client.response.parsed_body['next_since'].to_s,attachments:row.dig('message','drive_attachments')}
    else
      captured=[]
      # Keep outbound webhook local and recorded; no Google call is involved.
      RestrictedHTTP::PrivateNetworkGuard.define_singleton_method(:resolve) { |_host| '93.184.216.34' }
      WebMock.enable!
      WebMock.stub_request(:post,agent.user.webhook.url).to_return do |request|
        captured << JSON.parse(request.body);{status:200,body:''}
      end
      event=AgentEvent.find_by!(message:message,event_type:'mention')
      Agent::DeliveryJob.perform_now(event.id)
      Agent::EventWebhookJob.perform_now(event.id)
      raise "webhook did not post exactly once: #{event.reload.webhook_status} #{event.webhook_last_error}" unless captured.length==1
      result={attachments:captured[0].dig('message','drive_attachments'),agent_id:captured[0].dig('agent','id'),delivered:event.reload.webhook_status=='delivered'}
    end
    rows << {name:,ids:,result:,http:http_observation}
  rescue => error
    warn error.full_message
    raise
  ensure
    ActiveRecord::Base.connection_pool.disconnect!;FileUtils.rm_f(["#{database}-wal","#{database}-shm"]);FileUtils.cp(snapshot,database);FileUtils.rm_f(snapshot)
  end
end
puts JSON.pretty_generate({reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],rows:})
