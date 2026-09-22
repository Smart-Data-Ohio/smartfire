require "test_helper"
require "zlib"

class EmojiPickerDataTest < ActiveSupport::TestCase
  ASSET_PATH = Rails.root.join("app/assets/emoji/emoji.json")

  test "the checked-in asset matches a fresh generation" do
    assert_equal EmojiPickerData.generate + "\n", File.read(ASSET_PATH),
      "regenerate with bin/rails emoji_picker:generate after a gemoji upgrade"
  end

  test "the asset holds every gemoji character in the picker groups" do
    source = JSON.parse(File.read(EmojiPickerData.gemoji_db_path))
    asset = JSON.parse(File.read(ASSET_PATH))

    assert_equal %w[ smileys people nature food activities travel objects symbols flags ],
      asset["groups"].map { _1["id"] }

    asset_characters = asset["groups"].flat_map { |group| group["emoji"].map(&:first) }
    assert_equal source.map { _1["emoji"] }.sort, asset_characters.sort
    assert_not_empty asset_characters
  end

  test "every entry fits the boost content limit and carries a searchable name" do
    asset = JSON.parse(File.read(ASSET_PATH))

    asset["groups"].each do |group|
      group["emoji"].each do |character, aliases, description, _tags|
        assert_operator character.length, :<=, 16, "boost content for #{character}"
        assert_predicate aliases, :present?
        assert_predicate description, :present?
      end
    end
  end

  test "the asset stays small enough to fetch on first open" do
    payload = File.read(ASSET_PATH)
    gzipped = Zlib::Deflate.deflate(payload, Zlib::BEST_COMPRESSION).bytesize

    assert_operator gzipped, :<, 150 * 1024,
      "expected the emoji asset under 150 KB gzipped, was #{gzipped} bytes"
  end
end
