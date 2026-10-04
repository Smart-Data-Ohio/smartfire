# String#downcase from the pinned reference Ruby, including its Unicode data version.
require "json"
abort "wrong Rails runtime pin" unless ENV.fetch("PARITY_REFERENCE_SHA") == ARGV.fetch(0)
mappings = (0..0x10ffff).filter_map do |codepoint|
  next if (0xd800..0xdfff).cover?(codepoint)
  character = codepoint.chr(Encoding::UTF_8)
  lower = character.downcase
  [codepoint, lower] unless character == lower
end
puts JSON.generate(mappings)
