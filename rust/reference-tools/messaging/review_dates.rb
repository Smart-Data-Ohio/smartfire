require 'json'
inputs=['20260305 14:30','20260305 14:30 -0500','20260305143000 -0500','20260305143000 +0530','20260305143000Z','20260305143000 UTC','20260305T143000-0500','2026-03-05T14:30:00-05','2026-03-05T14:30:00+05','2026-03-05T14:30:00+05:30:30','2026-03-05T14:30:00-05:30:30','2026-03-05T14:30:00.123456789Z','2026-03-05T14:30:00-0500','2026-03-05T14:30:00Z','20260308 02:30','20260308023000','20260308 02:30 -0500','20261101 01:30','20261101013000','20261101013000 -0500','20260329023000','20261025023000']
cases=%w[UTC America/New_York Europe/Berlin Pacific/Apia].flat_map do |zone|
 Time.use_zone(zone) do
  inputs.map do |input|
   begin
    time=Time.zone.parse(input)
    {zone:,input:,parts:Date._parse(input,false),result:time&.utc&.iso8601(6),slash_result:SlashCommands::TimeParser.parse(input,zone:,now:Time.current)&.utc&.iso8601(6)}
   rescue ArgumentError,TypeError=>error
    {zone:,input:,parts:Date._parse(input,false),error:error.class.name}
   end
  end
 end
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',cases:)+"\n")
puts "WS8bm2 review date Rails oracle: #{cases.size} compact/offset/DST cases in 4 zones"
