require 'json'
cases=[['missing',{url:'https://www.example.com'}],['markup',{url:'https://www.example.com',title:%q{<img src='x' onerror='alert(document.domain)'/>},description:%q{<img src='x' onerror='alert(document.domain)'/>}}],['valid',{url:'https://www.example.com',title:'Hey!',description:'Hello'}]].map do |name,attributes|
 metadata=Opengraph::Metadata.new(attributes)
 valid=metadata.valid?
 {name:name,attributes:attributes,valid:valid,full_messages:metadata.errors.full_messages,json:metadata.to_json}
end
File.write(ARGV.fetch(0),JSON.pretty_generate({reference:ENV.fetch("PARITY_REFERENCE_SHA"),cases:cases})+"\n")
puts "WS15e OpenGraph validation Rails oracle: #{cases.size} cases"
