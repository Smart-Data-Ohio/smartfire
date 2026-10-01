require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026,3,2,16) do
  agent=Agent.find(773018776)
  room=Room.find(486777696)
  snapshot=->(message,claimed) do
    {claimed:claimed,streaming:message.reload.streaming?,references:message.github_pull_request_references.joins(:pull_request).order("github_pull_requests.number").pluck("github_pull_requests.number"),
     fetches:ApplicationJob.queue_adapter.enqueued_jobs.count { |j| j[:job]==Github::FetchPullRequestJob }}
  end
  ApplicationJob.queue_adapter.enqueued_jobs.clear
  message=room.root_messages.create!(creator:agent.user,streaming:true,markdown_source:"https://github.com/ws11-fixture/public/pull/1",client_message_id:"ws11-github-stream")
  result={start:snapshot.call(message,nil)}
  message.update!(markdown_source:"https://github.com/ws11-fixture/public/pull/2")
  result[:append]=snapshot.call(message,nil)
  result[:final]=snapshot.call(message,message.finalize_stream!)
  result[:repeat]=snapshot.call(message,message.finalize_stream!)
  quiet=room.root_messages.create!(creator:agent.user,streaming:true,markdown_source:"https://github.com/ws11-fixture/public/pull/3",client_message_id:"ws11-github-quiet")
  result[:quiet]=snapshot.call(quiet,quiet.finalize_stream_quietly!)
  puts JSON.pretty_generate(reference_pin:"d7c7de92",results:result)
end
