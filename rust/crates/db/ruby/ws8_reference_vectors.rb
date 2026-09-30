require "json"
# Expectations come from our Rails helpers, including HTML5 recovery and Unicode boundaries.
texts = [nil, "", "none", "/rooms/1/@2", "https://other.test/rooms/9/@2/path?q=1#x",
  "/rooms/1/@2thanks", "/rooms/1/@2é", "/rooms/1/@2_", "/rooms/1/@2-yes",
  "/rooms/1/@02 /rooms/1/@2", (1..12).map { |id| "/rooms/1/@#{id}" }.join(" ")]
htmls = ["<p>/rooms/1/@2</p><p>thanks</p>", "<code>/rooms/1/@2</code><p>/rooms/1/@3</p>",
  '<a href="/rooms/1/@4">labeled</a>', "<pre><a href='/rooms/1/@5'>code</a></pre><div>hello<br>world</div>",
  "<div>&amp; café /rooms/1/@2</div>", "<p>x<b>y</b></p><table><tr><td>z</td></tr></table>",
  "<p>broken<div>/rooms/1/@7</p></div>", "<!--/rooms/1/@8--><script>text</script>"]
File.write(ARGV.fetch(0), JSON.pretty_generate({
  extraction: texts.map { |text| { text: text, ids: Message::ReferenceSync.extract_message_ids(text) } },
  html: htmls.map { |html| { html: html, text: Message::ReferenceSync.non_code_text(html) } }
}) + "\n")
puts "WS8 reference vectors: #{texts.size} extraction, #{htmls.size} HTML cases"
