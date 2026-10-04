# The I512 boundary is shared infrastructure, not an accepted Rails difference.
# Capture real Rails output on both sides so the owner dependency is reproducible.
require 'json'
def encode(value)
 return value.utc.strftime('%Y-%m-%d %H:%M:%S.%6N') if value.respond_to?(:utc)
 return value.map { |item| encode(item) } if value.is_a?(Array)
 value
end
def observation
 {value:encode(yield)}
rescue StandardError => e
 {error:e.class.name,message:e.message}
end
cases=[]
limit=(1<<511)-1-(1<<100)
%w[UTC America/New_York].each do |zone|
 [[142,143,144,145,155,200],'hours'].then do |exponents,unit|
  exponents.each do |exponent|
   text="in 1#{'0'*exponent} #{unit}"
   parsed=SlashCommands::TimeParser.parse(text,zone:,now:Time.current)
   cases << {id:"#{zone}/#{exponent}_#{unit}",zone:,input:text,kind:'relative',fits_shared_timestamp:(parsed.to_r*1_000_000).floor.abs<=limit,
    parse:observation { SlashCommands::TimeParser.parse(text,zone:,now:Time.current) },
    leading:observation { SlashCommands::TimeParser.split_leading_time(text,zone:,now:Time.current) },
    trailing:observation { SlashCommands::TimeParser.split_trailing_time(text,zone:,now:Time.current) }}
  end
 end
 [[141,142,143,155,200],'days'].then do |exponents,unit|
  exponents.each do |exponent|
   text="in 1#{'0'*exponent} #{unit}"
   parsed=SlashCommands::TimeParser.parse(text,zone:,now:Time.current)
   cases << {id:"#{zone}/#{exponent}_#{unit}",zone:,input:text,kind:'relative',fits_shared_timestamp:(parsed.to_r*1_000_000).floor.abs<=limit,
    parse:observation { SlashCommands::TimeParser.parse(text,zone:,now:Time.current) },
    leading:observation { SlashCommands::TimeParser.split_leading_time(text,zone:,now:Time.current) },
    trailing:observation { SlashCommands::TimeParser.split_trailing_time(text,zone:,now:Time.current) }}
  end
 end
 ['', '-'].each do |sign|
  text="#{sign}#{'9'*110}-03-05 14:30:00"
  cases << {id:"#{zone}/#{sign.empty? ? 'positive' : 'negative'}_110_digit_calendar",zone:,input:text,kind:'calendar',fits_shared_timestamp:true,
   parse:observation { ActiveSupport::TimeZone[zone].parse(text,Time.current) }}
 end
end
File.write(ARGV.fetch(0), JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],now:Time.current.utc.iso8601,cases:)+"\n")
fits=cases.count { |c| c[:fits_shared_timestamp] }
puts "WS8bm2 extreme-range Rails: #{cases.size} actual parser cases; #{fits} within shared I512; #{cases.size-fits} outside shared I512, not approved differences"
