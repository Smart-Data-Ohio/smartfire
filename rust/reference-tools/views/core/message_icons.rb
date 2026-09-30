# The pinned registry's static facts for reaction titles and avatar shortcodes.
def message_icon_registry
  shortcodes = Icons.brands.each_with_object({}) do |brand, out|
    [brand.name, *brand.aliases].each do |name|
      out[name] = { title: brand.title, icon_alt: ":#{brand.name}:", icon: Icons.image_url_for(brand) && { "Image" => { title: brand.title, url: brand.logical_asset_path, brand: true } } }
    end
  end
  Emoji.all.each do |emoji|
    emoji.aliases.each do |name|
      title = Icons::Emoji.new(name: name, character: emoji.raw).title
      shortcodes[name] ||= { title: title, icon: { "Emoji" => { title: title, character: emoji.raw } } }
    end
  end
  { emoji_names: Emoji.all.each_with_object({}) { |emoji, out| emoji.unicode_aliases.each { |raw| out[raw] = emoji.name } }, shortcodes: shortcodes }
end
File.write('/rails/storage/db/message-icons.json', JSON.pretty_generate(message_icon_registry) + "\n")
puts "Rails message icon registry: #{message_icon_registry[:shortcodes].size} names"
