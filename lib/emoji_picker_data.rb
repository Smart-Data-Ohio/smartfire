# Generates the emoji picker's static data asset from the gemoji gem's
# vetted Unicode emoji list (MIT, Copyright (c) 2019 GitHub, Inc.).
#
# The asset ships as a digested Propshaft file and is fetched lazily the
# first time the picker opens, so it never loads with the page. Regenerate
# with `bin/rails emoji_picker:generate` after a gemoji upgrade.
module EmojiPickerData
  # gemoji category and picker tab id. Tab labels and glyphs live in the
  # picker partial; the asset carries only the emoji.
  GROUPS = [
    [ "Smileys & Emotion", "smileys" ],
    [ "People & Body", "people" ],
    [ "Animals & Nature", "nature" ],
    [ "Food & Drink", "food" ],
    [ "Activities", "activities" ],
    [ "Travel & Places", "travel" ],
    [ "Objects", "objects" ],
    [ "Symbols", "symbols" ],
    [ "Flags", "flags" ]
  ].freeze

  # Compact entries keep the asset small: [ character, aliases, description,
  # tags ], with aliases and tags space-joined for client-side search.
  def self.generate(source_path = gemoji_db_path)
    entries = JSON.parse(File.read(source_path))

    groups = GROUPS.map do |category, id|
      {
        id:,
        emoji: entries
          .select { |entry| entry["category"] == category }
          .map { |entry| [ entry["emoji"], entry["aliases"].join(" "), entry["description"], entry["tags"].join(" ") ] }
      }
    end

    JSON.generate({ version: 1, groups: })
  end

  def self.gemoji_db_path
    File.join(Gem.loaded_specs["gemoji"].full_gem_path, "db", "emoji.json")
  end
end
