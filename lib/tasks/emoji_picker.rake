namespace :emoji_picker do
  desc "Regenerate the emoji picker data asset from the gemoji gem"
  task generate: :environment do
    require "zlib"
    require "stringio"

    path = Rails.root.join("app/assets/emoji/emoji.json")
    payload = EmojiPickerData.generate + "\n"
    FileUtils.mkdir_p(path.dirname)
    File.write(path, payload)

    gzipped = Zlib::Deflate.deflate(payload, Zlib::BEST_COMPRESSION).bytesize
    puts "Wrote #{path} (#{payload.bytesize} bytes raw, #{gzipped} bytes gzipped)"
  end
end
