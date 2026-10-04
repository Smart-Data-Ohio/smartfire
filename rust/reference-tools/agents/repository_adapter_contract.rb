require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
Rails.cache = ActiveSupport::Cache::MemoryStore.new
travel_to Time.utc(2026,3,2,16) do
  cases={}
  agent=Agent.find(773018776);agent.update_columns(owner_id:127326141)
  thread=ChannelThread.create!(id:900130000,room_id:486777696,creator_id:127326141,name:"Private context")
  [true,nil,false].each_with_index do |visibility,index|
    id=900130001+index
    pr=Github::PullRequest.create!(id:id,owner:"Mixed",repo:"Repo",number:id,title:"Private title",private:visibility)
    WorkThreadLink.create!(channel_thread:thread,kind:"pull_request",github_pull_request:pr,created_by_id:127326141)
  end
  [200,403,404,401,500].each do |status|
    Rails.cache.clear
    GithubConnectedAccount.where(user_id:127326141).delete_all
    account=GithubConnectedAccount.create!(user_id:127326141,github_login:"ws11-owner",access_token:"ws11-public-fake-repository-token")
    paths=[]
    response=Net::HTTPResponse::CODE_TO_OBJ.fetch(status.to_s).new("1.1",status.to_s,"fixture")
    response.define_singleton_method(:body) {"{}"}
    http=Object.new
    http.define_singleton_method(:get) { |path,headers| paths << path; response }
    Net::HTTP.define_singleton_method(:start) { |*args,**kwargs,&block| block.call(http) }
    direct=account.can_read_repository?("Mixed","Repo")
    links=Agents::WorkPayload.for(thread,agent:agent.reload)[:links]
    decisions=[direct,links.first[:title].present?]
    cases[status]={decisions:decisions,paths:paths.dup,disconnected_reason:account.reload.disconnected_reason,links:links}
  end
  puts JSON.pretty_generate(reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],cases:cases)
end
