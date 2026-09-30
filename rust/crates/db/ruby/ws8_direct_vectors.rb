require "json"
cases = [
  { name: nil, names: ["David"], viewer: 0 },
  { name: nil, names: ["David", "Jason"], viewer: 0 },
  { name: nil, names: ["David", "Jason", "Kevin", "JZ", "Zed Last"], viewer: 0 },
  { name: "Weekend", names: ["David", "Jason"], viewer: 0 },
  { name: " ", names: ["David", "Jason"], viewer: 0 },
  { name: nil, names: ["Élodie Smith", "anna\u00a0bell", "bob"], viewer: nil },
  { name: nil, names: [], viewer: nil }
]
cases.each do |row|
  members = row[:names].map.with_index { |name,i| User.new(id:i+1,name:name) }
  room = Rooms::Direct.new(name:row[:name])
  row[:display] = room.direct_display_name(for_user: row[:viewer] && members[row[:viewer]], members:members)
end
File.write(ARGV.fetch(0),JSON.pretty_generate(cases)+"\n")
puts "WS8 direct vectors: #{cases.size} display cases"
