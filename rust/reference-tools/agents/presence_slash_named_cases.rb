require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter=:test
travel_to Time.utc(2026,3,2,16)
begin
  agent=Agent.find(773018776);room=Room.find(486777696)
  presence={}
  snapshot=-> do
    a=agent.reload
    {text:a.working_presence_text,stored:a.working_presence,expires_at:a.working_presence_expires_at}
  end
  agent.set_working_presence!("Running tests…")
  presence[:ttl]=snapshot.call
  agent.set_working_presence!("Thinking…");agent.set_working_presence!("")
  presence[:blank]=snapshot.call
  agent.set_working_presence!("Thinking…")
  travel(Agent::WORKING_PRESENCE_TTL+1.minute) {presence[:expired]=snapshot.call}
  agent.working_presence="x"*141;agent.valid?
  presence[:limit]=agent.errors.to_hash
  agent.reload
  service=->(text) do
    r=Agents::WorkingPresence.set(agent:agent,text:text)
    {status:r.status==:unprocessable_entity ? 422 : Rack::Utils::SYMBOL_TO_STATUS_CODE.fetch(r.status),payload:r.payload,error:r.error}
  end
  presence[:service]=[service.call("Thinking…"),service.call("")]
  presence[:service_limit]=service.call("x"*141)
  agent.agent_slash_commands.delete_all
  commands={}
  c=AgentSlashCommand.create!(agent:agent,room:room,name:"Deploy",description:" Ship it ")
  commands[:register]={name:c.name,description:c.description,takes_arguments:c.takes_arguments}
  other=Agent.create!(user:User.create_bot!(name:"Other Bot"),owner_id:127326141,kind: :workspace)
  dup=AgentSlashCommand.new(agent:other,room:room,name:"deploy");dup.valid?
  commands[:duplicate]=dup.errors.to_hash
  commands[:invalid]=["Deploy!","/deploy","two words","x"*33].map do |name|
    c=AgentSlashCommand.new(agent:agent,room:room,name:name);c.valid?;c.errors.to_hash
  end
  %w[builtin description].each do |kind|
    c=AgentSlashCommand.new(agent:agent,room:room,name:kind=="builtin" ? "poll" : "inspect",description:kind=="description" ? "x"*141 : nil)
    c.valid?;commands[kind]=c.errors.to_hash
  end
  puts JSON.pretty_generate({reference_pin:"d7c7de92",presence:presence,commands:commands}.as_json)
end
