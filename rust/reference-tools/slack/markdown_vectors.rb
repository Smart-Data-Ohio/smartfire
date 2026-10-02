# Run in our pinned Rails app. Pure converter vectors; no HTTP or writes.
require 'json'
require 'digest'
users = { 'U001' => 'Jane Doe', 'U002' => 'Kevin', 'U003' => 'Bad[Name]', 'U004' => "Two\nLines", 'U005' => '  ', 'U006' => 'Zoë' }
texts = [
  'fish &amp; chips &lt;tag&gt;', 'hi <@U001>!', 'hi <@U001|jane>!',
  'hi <@U999|ghost>!', 'hi <@U999>!', 'see <#CPRIV|secret>', 'see <#CPRIV>',
  '<!here> standup', '<!channel> news', '<!everyone> hi', '<!here|@here> standup',
  '<!channel|channel> news', '<!everyone|everyone> hi', 'ping <!subteam^S123|@engs>',
  '<!subteam^S123>', 'due <!date^1700000000^{date_short}|Feb 1>',
  'due <!date^1700000000^{date_short}^https://example.com|Feb 1>', 'due <!date^1700000000^{date_short}>',
  'see <https://example.com|docs>', 'see <https://example.com>',
  'write <mailto:team@example.com|us>', 'write <mailto:team@example.com>',
  'a *bold* word', 'an _italic_ word', 'a ~struck~ word', 'var snake_case_name here',
  '`*literal*` and *bold*', "```\n*block*\n```", '```code```', "```const x = 1\nputs x\n```",
  'see <https://example.com/a*b_c|a*b_c>', 'see <https://example.com/*path*/x_y>',
  'see https://example.com/*path*/x_y and *bold*', '• item', 'nice :wave::skin-tone-2: done',
  '', 'hello', '<@U003> <@U004> <@U005> <@U006>', '**bold** *a*b* ~~x~~',
  "é_italic_é _x_", "a\n \n• b\n\t• c", 'SMARTFIRESLACKLINK0END <https://example.com|x>',
  '`unclosed <@U001> *bold*', '```unclosed *bold*'
]
cases = texts.map { |text| { 'message' => { 'text' => text } } }
cases += [
  { 'message' => { 'text' => 'see this', 'files' => [ { 'name' => 'spec.pdf', 'permalink' => 'https://files.example/s.pdf' }, { 'title' => 'shot', 'permalink_public' => 'https://files.example/shot' }, { 'name' => 'x' }, { 'url_private' => 'https://files.example/private' } ] } },
  { 'message' => { 'text' => '', 'attachments' => [ { 'pretext' => 'Build', 'text' => 'All green', 'fallback' => 'Build passed' } ] } },
  { 'message' => { 'text' => 'Build *passed*', 'subtype' => 'bot_message', 'bot_id' => 'B1', 'attachments' => [ { 'text' => 'All green' } ] } },
  { 'message' => { 'text' => 'look', 'attachments' => [ { 'text' => 'unfurl' } ] } },
  { 'message' => { 'text' => "waves hello\n\nnext\n", 'subtype' => 'me_message' } },
  { 'message' => { 'text' => nil } }, { 'message' => { 'text' => 123 } },
  { 'message' => { 'text' => " \n", 'attachments' => [ { 'text' => " a\n\n b\n" } ] } }
]
fixtures = Dir[Rails.root.join('test/fixtures/files/slack/*.json')].sort
fixtures.each do |path|
  JSON.parse(File.read(path)).fetch('messages', []).each { |message| cases << { 'message' => message } }
end
random = Random.new(16)
atoms = texts + [ '*', '_', '~', '`', "\n", 'é', '🙂', ' ', "\r\n", '<@U001|*a*>' ]
400.times { cases << { 'message' => { 'text' => Array.new(random.rand(2..10)) { atoms.sample(random: random) }.join } } }
[ [ 'x', 50_100 ], [ '🙂', 50_001 ], [ 'é', 49_999 ], [ '<!date^' + '!^' * 50_000, 1 ] ].each do |unit, count|
  cases << { 'repeat' => { 'unit' => unit, 'count' => count }, 'message' => {} }
end
cases.each do |entry|
  message = entry.fetch('message').dup
  message['text'] = entry['repeat']['unit'] * entry['repeat']['count'] if entry['repeat']
  result = Slack::MarkdownConverter.convert(message, users: users)
  entry['expected'] = { 'markdown' => result.markdown, 'truncated' => result.truncated, 'files_linked' => result.files_linked }
end
output = { 'reference' => 'd7c7de9264c63015be398001d7a1094e7695a6db',
  'source_sha256' => Digest::SHA256.file(Rails.root.join('app/models/slack/markdown_converter.rb')).hexdigest,
  'users' => users, 'cases' => cases }
File.write(File.join(ENV.fetch('PARITY_WORK'), 'vectors/slack/markdown.json'), JSON.pretty_generate(output) + "\n")
puts "Slack markdown vectors: #{cases.size} cases generated from Rails"
