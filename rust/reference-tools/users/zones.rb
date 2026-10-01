require "json"
identifiers = TZInfo::Timezone.all_identifiers.sort
if ARGV.first == "named"
  names = ActiveSupport::TimeZone::MAPPING.keys.to_h { |name| [name, ActiveSupport::TimeZone[name]&.tzinfo&.identifier] }
  puts JSON.pretty_generate(names)
  warn "Rails named-zone oracle: #{names.length} names, #{names.values.count(nil)} unavailable; reference d7c7de92"
else
  puts JSON.pretty_generate(identifiers)
  warn "Rails zone oracle: #{identifiers.length} case-sensitive TZInfo identifiers; reference d7c7de92"
end
