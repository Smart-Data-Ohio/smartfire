# Focused PR #223 boundaries, measured with the real pinned TimeParser and renderers.
require 'json'
now = Time.current
inputs = %w[UTC America/New_York].flat_map do |zone|
  [140, 141, 142].product(%w[days weeks]).map do |exponent, unit|
    {id: "#{zone}/#{exponent}_#{unit}", zone:, input: "in 1#{'0' * exponent} #{unit}"}
  end
end
inputs << {id: 'UTC/142_hours', zone: 'UTC', input: "in 1#{'0' * 142} hours"}
def observation
  value = yield
  encode = ->(v) { v.is_a?(Time) || v.is_a?(ActiveSupport::TimeWithZone) ? v.utc.strftime('%Y-%m-%d %H:%M:%S.%6N') : v }
  {value: value.is_a?(Array) ? value.map { |v| encode.call(v) } : encode.call(value)}
rescue StandardError => error
  {error: error.class.name, message: error.message}
end
cases = inputs.map do |c|
  text = c.fetch(:input)
  zone = c.fetch(:zone)
  parsed = SlashCommands::TimeParser.parse(text, zone:, now:)
  local = parsed.in_time_zone(zone)
  c.merge(now: now.utc.iso8601,
    parse: observation { SlashCommands::TimeParser.parse(text, zone:, now:) },
    leading: observation { SlashCommands::TimeParser.split_leading_time(text, zone:, now:) },
    trailing: observation { SlashCommands::TimeParser.split_trailing_time(text, zone:, now:) },
    render: local.iso8601, json: local.as_json,
    since: (parsed + Rational(1001, 1_000_000_000)).utc.strftime('%Y-%m-%d %H:%M:%S.%6N'),
    ago: (parsed - Rational(1001, 1_000_000_000)).utc.strftime('%Y-%m-%d %H:%M:%S.%6N'))
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: 'd7c7de92', cases:) + "\n")
puts "PR223 overflow Rails: #{cases.length * 3} parser/split outcomes; #{cases.length} render/JSON/offset boundaries"
