require 'json'
inputs=['0305143000','260305143000','20260305143000.5','20260305143000.123456789 -0500','20260305143000.5Z','20260305143000+05','20260305143000+05:30','20260305143000+05:30:30','2026064','202603051','260305 14:30 -0500','0305143000-0500','260305143000-0500','20260305T1430','2026-03-05 143000 -0500','20260305 143000 -0500','20260308T023000','20261101T013000','0308023000','1101013000','20260305143000 -05','20260305143000 GMT+5','20260305143000 UTC+5','20260305143000 PST']
cases=%w[UTC America/New_York Europe/Berlin Pacific/Apia].flat_map do |zone|
 Time.use_zone(zone) do
  inputs.map do |input|
   begin
    time=Time.zone.parse(input)
    {zone:,input:,parts:Date._parse(input,false),result:time&.utc&.iso8601(6)}
   rescue ArgumentError,TypeError=>error
    {zone:,input:,parts:Date._parse(input,false),error:error.class.name}
   end
  end
 end
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],cases:)+"\n")
puts "WS8bm2 compact width Rails oracle: #{cases.size} short/ordinal/fraction/compact-clock cases in 4 zones"
