require 'json'
inputs=['','not a date','March 5, 2026 14:30','5 March 2026','2026/03/05','05/03/2026','March 2027','2026-02-30','2026-03-08 02:30','2026-11-01 01:30','2026-03-05T24:00:00','2026-03-05T12:30:60','2026-03-05T12:30:05.123456789Z','Thu, 05 Mar 2026 14:30:00 -0500','20260305','20260305143000','2026-15-01','2026-03-32','25:30','12:61','tomorrow','sunday','17:00 MART']
cases=%w[UTC America/New_York Pacific/Apia].flat_map do |zone|
  Time.use_zone(zone) do
    inputs.map do |input|
      begin
        time=Time.zone.parse(input)
        {zone:,input:,result:time&.utc&.iso8601(6)}
      rescue ArgumentError, TypeError=>error
        {zone:,input:,error:error.class.name}
      end
    end
  end
end
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',cases:)+"\n")
puts "WS8bm2 date Rails oracle: #{cases.size} Time.zone.parse cases in 3 zones"
