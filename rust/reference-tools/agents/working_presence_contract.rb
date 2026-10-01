require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter = :test
travel_to Time.utc(2026, 3, 2, 16) do
  agent = Agent.find(773018776)
  results = {}
  [[:set, " Thinking… "], [:clear, " \t\0"], [:nil, nil],
   [:boundary, "é" * 140], [:oversized, "é" * 141]].each do |name, text|
    result = Agents::WorkingPresence.set(agent: agent, text: text)
    code = result.status == :unprocessable_entity ? 422 : Rack::Utils::SYMBOL_TO_STATUS_CODE.fetch(result.status)
    results[name] = {status: code, payload: result.payload, error: result.error,
      stored: Agent.find(agent.id).working_presence, expires_at: Agent.find(agent.id).working_presence_expires_at}
  end
  puts JSON.pretty_generate({ reference_pin: "d7c7de92", results: results }.as_json)
end
