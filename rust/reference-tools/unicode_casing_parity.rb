# Record Ruby/SQLite expectations for the seven Unicode casing scout findings.
# Run: docker run --rm -i --entrypoint bash campfire-reference \
#   -c 'bundle exec ruby -rjson -rrbconfig -rsqlite3' \
#   < reference-tools/unicode_casing_parity.rb > vectors/unicode_casing_parity.json
raise "Ruby runtime drift" unless RUBY_REVISION == "2b0b7728dc7f0561c35c3d8c4489945c94b783ad"

# Authentication-Results operations from app/mailboxes/room_mailbox.rb at d7c7de92.
auth_cases = [
  ["mx.mail.test; dkim=pass header.d=Éxample.com", "mx.mail.test", "alice@éxample.com"],
  ["mx.mail.test; dmarc=pass header.from=οσ.example", "mx.mail.test", "alice@ΟΣ.example"],
  ["mx.mail.test; spf=pass smtp.mailfrom=bob@ΟΣ.example", "mx.mail.test", "alice@οσ.example"],
  ["mx.ß.example; dkim=pass header.d=example.com", "mx.ss.example", "alice@example.com"],
  ["mx.ς.example; dkim=pass header.d=example.com", "mx.σ.example", "alice@example.com"],
  ["mx.mail.test; dkim=pass header.d=ss.example", "mx.mail.test", "alice@ß.example"],
  ["mx.mail.test; dkim=pass header.d=ς.example", "mx.mail.test", "alice@σ.example"]
].map do |header, relay, address|
  domain = address.split("@").last.to_s.downcase
  trusted = header.split(";").first.to_s.strip.casecmp?(relay)
  expected = trusted && header.split(";").drop(1).any? do |clause|
    method, rest = clause.strip.split("=", 2).map { |part| part.to_s.strip.downcase }
    next false unless rest.to_s.split(/[\s(]/, 2).first == "pass"
    properties = {"dkim" => %w[header.d], "dmarc" => %w[header.from], "spf" => %w[smtp.mailfrom]}[method]
    next false unless properties
    pattern = properties.map { |property| Regexp.escape(property) }.join("|")
    clause.scan(/(?:#{pattern})=([^\s;()]+)/i).flatten
      .map { |value| value.downcase.sub(/\A@/, "").split("@").last }.include?(domain)
  end
  {header:, relay:, address:, expected:}
end

db = SQLite3::Database.new(":memory:")
db.execute("CREATE TABLE users(name TEXT)")
sql_names = ["a\0z", "A\0a", "É", "é"]
sql_names.each { |name| db.execute("INSERT INTO users VALUES (?)", [name]) }
strings = ["ΟΣ", "ΟΣ@example.com", "ΟΣ@local", "Éxample.com", "ß", "ς", "σ", "İ", "a\0Z", "\u1c89"]
pairs = [["ß", "SS"], ["ς", "σ"], ["ΟΣ", "οσ"], ["É", "é"], ["é", "e\u0301"], ["a\0z", "A\0Z"]]
names = ["ΟΣ", "οςa", "É", "é", "ß", "SS"]
puts JSON.pretty_generate(
  reference: "d7c7de92", ruby: RUBY_DESCRIPTION,
  unicode: RbConfig::CONFIG.fetch("UNICODE_VERSION"), sqlite: SQLite3::SQLITE_VERSION,
  casing: strings.map { |input| {input:, downcase: input.downcase, fold: input.downcase(:fold)} },
  comparisons: pairs.map { |left, right| {left:, right:, equal: left.casecmp?(right)} },
  auth: auth_cases,
  names: {input: names, sorted: names.sort_by(&:downcase)},
  sigma_names: ["ΟΣ", "οςa"].sort_by(&:downcase),
  sql_names: {input: sql_names, sorted: db.execute("SELECT name FROM users ORDER BY LOWER(users.name)").flatten}
)
