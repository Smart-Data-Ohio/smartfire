# Use Rails' real documented grammar, including overflow and whole-string coercion.
require 'json'
now = Time.current
inputs = ['in 0 minutes', 'in 1 minute', 'in 9223372036854775807 minutes', 'in 9223372036854775808 minutes', 'in 999999999999999999999999999 hours', 'in 2147483648 days', 'in 3652059 days', 'in 999999999999999999999 weeks', 'in 2 hr', 'in 2 hrs', 'in 2min', 'in 2 mins', 'in 2 days suffix', 'in 2 weeks\nbody', '2026-03-05 25:00 suffix', '2026-03-05 14:60 title', '2026-03-05 14:30:60 tail', '2026-03-05 14:30 +05:30 suffix', 'tomorrow 24:00 body', 'today at 9pm body', 'at 12pm text', 'at 0pm body', 'monday 24:00 tail', 'next monday junk', 'next monday 8pm title', 'xmonday', "\0tomorrow\0", "\u00a0tomorrow\u00a0", "\tfriday\n", 'review tomorrow at 9am', 'reviewfriday', 'review in 2 days', 'review 2026-03-05 14:30', 'friday', '', '   ']
containers = [nil, false, true, 17, [], ['tomorrow'], ['tomorrow', 'note'], {}, {'at' => 'tomorrow'}, {'nested' => [nil, 'friday']}]
def observation
  value = yield
  encode = ->(v) { v.is_a?(Time) || v.is_a?(ActiveSupport::TimeWithZone) ? v.utc.strftime('%Y-%m-%d %H:%M:%S.%6N') : v }
  {value: value.is_a?(Array) ? value.map { |v| encode.call(v) } : encode.call(value)}
rescue StandardError => error
  {error: error.class.name, message: error.message}
end
cases = %w[UTC America/New_York Australia/Lord_Howe Pacific/Apia].flat_map do |zone|
  (inputs + containers).map do |input|
    text = input.is_a?(String) ? input.gsub('\\n', "\n") : input
    {zone:, input: text,
     parse: observation { SlashCommands::TimeParser.parse(text, zone:, now:) },
     leading: observation { SlashCommands::TimeParser.split_leading_time(text, zone:, now:) },
     trailing: observation { SlashCommands::TimeParser.split_trailing_time(text, zone:, now:) }}
  end
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], cases:) + "\n")
puts "WS8bm2 relative/split Rails: #{cases.length} inputs; #{cases.length * 3} actual parser/split outcomes and exceptions"
