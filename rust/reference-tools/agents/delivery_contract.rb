require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16) do
  agent=Agent.find(773018776)
  message=Message.find(136976342)
  statuses=[201,200,404,302,408,429,500]
  status=nil; hint=nil
  Webhook.define_method(:post_payload) do |_payload, secret: nil|
    response=Net::HTTPResponse::CODE_TO_OBJ.fetch(status.to_s).new("1.1",status.to_s,"Fixture")
    response["Retry-After"]=hint if hint
    response
  end
  snapshots=statuses.map do |code|
    status=code;hint="600"
    e=agent.agent_events.create!(event_type:"mention",room:message.room,message:message,outcome:"delivered",webhook_status:"pending")
    Agent::EventWebhookJob.perform_now(e.id,0)
    e.reload
    {status:code,webhook_status:e.webhook_status,attempts:e.webhook_attempts,last_error:e.webhook_last_error,next_delay:e.webhook_next_attempt_at-Time.current}
  end
  hints=[nil,"45","9999999999999999999999999"," 0 ","-1","broken","Mon, 02 Mar 2026 16:00:45 GMT","Mon, 02 Mar 2026 15:00:00 GMT"]
  delays=hints.map do |value|
    status=429;hint=value
    e=agent.agent_events.create!(event_type:"mention",room:message.room,message:message,outcome:"delivered",webhook_status:"pending")
    Agent::EventWebhookJob.perform_now(e.id,0)
    {hint:value,delay:e.reload.webhook_next_attempt_at-Time.current}
  end
  status=500;hint=nil
  e=agent.agent_events.create!(event_type:"mention",room:message.room,message:message,outcome:"delivered",webhook_status:"pending")
  exhaustion=5.times.map do |attempt|
    Agent::EventWebhookJob.perform_now(e.id,attempt)
    e.reload
    {attempts:e.webhook_attempts,status:e.webhook_status,delay:e.webhook_next_attempt_at-Time.current}
  end
  puts JSON.pretty_generate(reference_pin:"d7c7de92",responses:snapshots,retry_after:delays,exhaustion:exhaustion)
end
