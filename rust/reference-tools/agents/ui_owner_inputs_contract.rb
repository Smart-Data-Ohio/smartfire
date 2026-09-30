# WS11 owner APIs: actual model writes and raw validation/form values at d7c7de92.
ApplicationJob.queue_adapter = :test
require 'json'
inputs = [nil, '', ' ', '12', ' 12 ', '+12', '012', '12.0', '12.5', '12e1', '12x', 'x12', 'abc', '0', '-1', '1_000', '0x10', '9223372036854775807', '9223372036854775808', 12, 12.0, 12.5, true, false, [], ['12'], {'value'=>'12'}]
bot = User.find_by!(name: 'Bender Bot')
owner = User.find_by!(email_address: 'david@37signals.com')
result = {reference: 'd7c7de9264c63015be398001d7a1094e7695a6db'}
result[:caps] = %w[daily_message_cap daily_board_post_cap daily_external_action_cap].flat_map do |field|
  inputs.map do |input|
    permitted = ActionController::Parameters.new(agent: {field=>input}).permit(agent: [field])[:agent]
    agent = Agent.new(user: bot, owner: owner, **permitted.to_h)
    agent.valid?
    {field: field, input: input, permitted: permitted.key?(field), value: agent.public_send(field), before_type_cast: agent.public_send("#{field}_before_type_cast"), errors: agent.errors[field]}
  end
end
result[:icons] = [nil, '', ' ', ':', '::', ':openai:', ' ::: OpenAI ::: ', 'OPENAI', 'openai', ':robot:', 'notanicon', ':notanicon:', ' open ai ', '😀', "\u00a0:openai:\u00a0", 12, true, false, [], ['openai'], {'name'=>'openai'}].map do |input|
  permitted = ActionController::Parameters.new(user: {icon_name: input}).require(:user).permit(:icon_name)
  user = User.active_bots.new(name: 'Icon contract', **permitted.to_h)
  user.valid?
  {input: input, permitted: permitted.key?(:icon_name), value: user.icon_name, errors: user.errors[:icon_name]}
end
result[:brand_names] = Icons.brands.flat_map { |b| [b.name, *b.aliases] }.sort
bot.update_columns(icon_name: nil, updated_at: 2.days.ago)
before = bot.updated_at
bot.update!(icon_name: ':OpenAI:')
result[:icon_write] = {value: bot.icon_name, timestamp_changed: bot.updated_at != before}
before = bot.updated_at
bot.update!(icon_name: ' ::: OPENAI ::: ')
result[:icon_noop] = {value: bot.icon_name, timestamp_changed: bot.updated_at != before}
bot.update!(icon_name: '')
result[:icon_clear] = bot.icon_name
result[:unknown_create] = begin
  User.create_bot!(name: 'Invalid icon', icon_name: ':notanicon:')
rescue ActiveRecord::RecordInvalid => e
  e.record.errors[:icon_name]
end
result[:unknown_update] = {saved: bot.update(icon_name: ':notanicon:'), errors: bot.errors[:icon_name], persisted: bot.reload.icon_name}
agent = bot.agent
result[:secrets] = [nil, '', ' ', 'fixture-read-only-secret'].map do |value|
  agent.update!(webhook_signing_secret: value)
  before = Agent.where(id: agent.id).pick(:updated_at)
  cipher = agent.read_attribute_before_type_cast(:webhook_signing_secret)
  values = 3.times.map {agent.reload.webhook_signing_secret}
  {input: value, values: values, ciphertext_unchanged: agent.read_attribute_before_type_cast(:webhook_signing_secret) == cipher, timestamp_unchanged: agent.updated_at == before}
end
puts JSON.pretty_generate(result)
