# Rails-owned input semantics for the pending WS11 icon/raw-parameter seams.
# This corpus is an oracle, not a claim that these mutations pass in Rust.
require_relative "setup"

bot = User.find_by!(name: "Bender Bot")
owner = User.find_by!(email_address: "david@37signals.com")
result = { "reference" => ENV.fetch("PARITY_REFERENCE_SHA") }

inputs = [nil, "", " ", "12", " 12 ", "+12", "012", "12.0", "12.5", "12e1",
  "12x", "x12", "abc", "0", "-1", "1_000", "0x10", "9223372036854775807",
  "9223372036854775808", 12, 12.0, 12.5, true, false, [], ["12"], { "value" => "12" }]
result["caps"] = %w[ daily_message_cap daily_board_post_cap daily_external_action_cap ].flat_map do |field|
  inputs.map do |input|
    permitted = ActionController::Parameters.new(agent: { field => input }).permit(agent: [field])[:agent]
    agent = Agent.new(user: bot, owner:, **permitted.to_h)
    agent.valid?
    { field:, input:, permitted: permitted.key?(field), value: agent.public_send(field), errors: agent.errors[field] }
  end
end

result["icons"] = [nil, "", " ", ":", "::", ":openai:", " ::: OpenAI ::: ", "OPENAI", "openai",
  ":robot:", "notanicon", ":notanicon:", " open ai ", "😀", "\u00a0:openai:\u00a0", 12, true, false, [], ["openai"], { "name" => "openai" }].map do |input|
  permitted = ActionController::Parameters.new(user: { icon_name: input }).require(:user).permit(:icon_name)
  user = User.active_bots.new(name: "Icon contract", **permitted.to_h)
  user.valid?
  { input:, permitted: permitted.key?(:icon_name), value: user.icon_name, errors: user.errors[:icon_name] }
end

date_inputs = [nil, "", " ", "invalid", "2030-06-15T10:20", "2030-06-15T10:20:30",
  "2030-06-15T10:20:30Z", "2030-06-15T10:20:30+05:30", "2030-06-15", "20300615",
  "06/15/2030", "15 Jun 2030 10:20:30", "2030-06-15 10:20:30.123456",
  "2026-03-08T02:30:00", "2026-11-01T01:30:00", 12, true, false, [], ["2030-06-15"], { "year" => 2030 }]
result["expiry"] = ["UTC", "America/New_York"].flat_map do |zone|
  Time.use_zone(zone) do
    date_inputs.map do |input|
      permitted = ActionController::Parameters.new(agent_credential: { expires_at: input }).require(:agent_credential).permit(:expires_at)
      credential = AgentCredential.new(expires_at: permitted[:expires_at].presence)
      value = credential.expires_at
      { zone:, input:, permitted: permitted.key?(:expires_at), value_class: value.class.name,
        value: value.respond_to?(:utc) ? value.utc.iso8601(6) : value }
    end
  end
end

File.write("/rails/storage/db/bot-input-contract.json", JSON.pretty_generate(result) + "\n")
puts "Rails bot input contract: #{result['caps'].size} cap, #{result['icons'].size} icon, #{result['expiry'].size} expiry cases"
