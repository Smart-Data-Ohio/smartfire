# Both branches of agents/work_controller_test.rb:88, through the real serializer.
require 'digest'
require 'action_dispatch/testing/integration'
require 'active_support/testing/time_helpers'
require 'webmock'
include WebMock::API
WebMock.enable!
extend ActiveSupport::Testing::TimeHelpers
{
  'test/controllers/agents/work_controller_test.rb'=>'86dd265c9cad2304af7fb32e3e2c376183387e3edb164e989258cd11704f5771',
  'app/models/agents/work_payload.rb'=>'1f082e78219532557dffc654b10b75efc9ab305cdf55598097965579f0cc01d7'
}.each { |file,hash| raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest == hash }
ApplicationController.allow_forgery_protection = false
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026,3,2,16) do
  agent = Agent.find(773018776)
  agent.agent_events.delete_all; agent.agent_grants.delete_all; agent.agent_credentials.delete_all
  agent.update_columns(status:'idle',owner_id:127326141)
  secret = 'ws11api-fixture-credential'
  agent.agent_credentials.create!(name:'HTTP contract',created_by_id:127326141,token_digest:AgentCredential.digest(secret),token_last_four:AgentCredential.digest(secret).first(4))
  %w[read_messages post_messages].each { |cap| AgentGrant.create!(agent:,capability:cap,granted_by_id:127326141) }
  GithubConnectedAccount.where(user_id:127326141).delete_all
  ActiveRecord::Base.connection.execute("UPDATE sqlite_sequence SET seq=1900700000 WHERE name='messages'")
  Message.create!(room_id:486777696,creator_id:127326141,markdown_source:'Source α & β',client_message_id:'read-source')
  Message.create!(room_id:486777696,creator_id:394959859,markdown_source:'Agent',client_message_id:'read-agent')
  ActiveRecord::Base.connection.execute("INSERT INTO channel_threads(id,name,room_id,creator_id,work_owner_id,work_status,last_activity_at,created_at,updated_at) VALUES(1900700020,'Owned α & β',486777696,394959859,394959859,'in_progress','2026-03-02 16:00:00','2026-03-02 16:00:00','2026-03-02 16:00:00')")
  ThreadTag.create!(channel_thread_id:1900700020,name:'api')
  pr = Github::PullRequest.create!(id:1900700030,owner:'acme',repo:'secret',number:3,title:'Secret acquisition α & β',state:'open',head_branch:'secret-branch',base_branch:'main',review_decision:'approved',check_status:'passing',private:true)
  WorkThreadLink.create!(id:1900700031,channel_thread_id:1900700020,kind:'pull_request',github_pull_request:pr,created_by_id:127326141)
  Message.create!(room_id:486777696,creator_id:394959859,thread_id:1900700020,markdown_source:'Thread',client_message_id:'read-thread')
  browser = ActionDispatch::Integration::Session.new(Rails.application)
  browser.host! 'campfire.test'
  steps = %w[disconnected connected].map do |name|
    if name == 'connected'
      GithubConnectedAccount.create!(id:1996100000,user_id:127326141,github_login:'owner-gh',access_token:'obviously-fake-owner-token')
      stub_request(:get,'https://api.github.com/repos/acme/secret').to_return(status:200,body:'{}')
    end
    browser.get('/agents/work/1900700020',headers:{'Authorization'=>['Bearer',secret].join(' '),'Accept'=>'application/json'})
    {name:,status:browser.response.status,body:browser.response.body,headers:%w[content-type cache-control pragma location].to_h{|key|[key,browser.response.headers[key]]}}
  end
  puts JSON.pretty_generate(reference:'agents/work_controller_test.rb:88',setup:{grant:%w[read_messages post_messages],work_pr:true,pr_private:true},steps:)
end
warn 'WS12_PRIVATE_PR_OWNER_RAILS 2 HTTP responses; disconnected redaction and connected-owner title/branch; 0 masks'
