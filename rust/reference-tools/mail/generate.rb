require 'json'
require_relative 'review_fixtures'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
ENV['INBOUND_EMAIL_AUTHSERV_ID'] = 'mx.mail.test'
ENV['MAILER_FROM'] = 'Smartfire <alerts@example.com>'
Rails.application.routes.default_url_options.merge!(host: 'example.com', protocol: 'https', port: 443)
mailbox = RoomMailbox.new(nil)
headers = [['=?UTF-8?B?bXgubWFpbC50ZXN0?=; dkim=pass header.d=example.com'], ['mx.mail.test; dkim=pass header.d==?UTF-8?B?ZXhhbXBsZS5jb20=?='], [], ['attacker.test; dkim=pass header.d=example.com'], ['mx.mail.test; dkim=fail header.d=example.com', 'mx.mail.test; dkim=pass header.d=example.com'], ['attacker.test; dkim=pass header.d=example.com', 'mx.mail.test; dkim=fail header.d=example.com']]
%w[dkim dmarc spf arc].product(%w[pass fail neutral PASS], ['header.d', 'header.from', 'smtp.mailfrom', 'smtp.helo'], ['example.com', '@example.com', 'a@example.com', 'evil.test', 'example.com.evil.test', '(example.com)']).each do |method, result, prop, domain|
  headers << ["mx.mail.test; #{method}=#{result} (comment) #{prop}=#{domain}"]
end
['mx.mail.test (comment); dkim=pass header.d=example.com', ' MX.MAIL.TEST ; DKIM = pass header.d=EXAMPLE.COM', 'mx.mail.test; dkim=pass(foo) header.d=example.com', 'mx.mail.test; dkim=pass xheader.d=example.com', 'mx.mail.test; dkim=pass header.d =example.com'].each { |h| headers << [h] }
whitespace = [0, 9, 10, 11, 12, 13, 32, 0x85, 0xa0, 0x1680, 0x2000, 0x2007, 0x2028, 0x202f, 0x205f, 0x3000].map do |codepoint|
  char = codepoint.chr(Encoding::UTF_8)
  ["#{char}mx.mail.test; dkim=pass header.d=example.com", "mx.mail.test;#{char}dkim=pass header.d=example.com", "mx.mail.test; dkim=#{char}pass header.d=example.com", "mx.mail.test; dkim=pass#{char}header.d=example.com", "mx.mail.test; dkim=pass header.d=example.com#{char}evil.test"].each { |h| headers << [h] }
  {value: char, stripped: "#{char}text#{char}".strip, blank: char.blank?, integer: "#{char}25suffix".to_i}
end
['mx.mail.test; dkim=pass (header.d=example.com) header.d=attacker.test', 'mx.mail.test; dkim=pass reason="header.d=example.com test" header.d=attacker.test'].each { |h| headers << [h] }
rng = Random.new(10)
100.times do
  headers << ["#{['mx.mail.test', 'foreign.test'].sample(random: rng)}; #{['dkim', 'dmarc', 'spf'].sample(random: rng)}=#{['pass','fail'].sample(random: rng)} #{['header.d','header.from','smtp.mailfrom','smtp.helo'].sample(random: rng)}=#{['example.com','evil.test'].sample(random: rng)}"]
end
auth = headers.map do |fields|
  source = (["From: person@example.com", "To: room-token@mail.test"] + fields.map { |h| "Authentication-Results: #{h}" } + ['', 'Hello']).join("\r\n")
  parsed = Mail.read_from_string(source)
  mailbox.define_singleton_method(:mail) { parsed }
  { headers: fields, parsed_headers: parsed.header.fields.select { |f| f.name.casecmp?('Authentication-Results') }.map { |f| f.value.to_s }, authserv_id: ENV['INBOUND_EMAIL_AUTHSERV_ID'], address: 'person@example.com', expected: mailbox.send(:authenticated_sender?, 'person@example.com') }
end
JSON.parse(File.read('/tools/corpus/public-auth-headers.json')).each do |sample|
  ENV['INBOUND_EMAIL_AUTHSERV_ID'] = sample.fetch('authserv_id')
  source = (["From: #{sample.fetch('address')}", 'To: room-token@mail.test'] + sample.fetch('headers').map { |h| "Authentication-Results: #{h}" } + ['', 'Hello']).join("\r\n")
  parsed = Mail.read_from_string(source)
  mailbox.define_singleton_method(:mail) { parsed }
  auth << sample.merge('expected' => mailbox.send(:authenticated_sender?, sample.fetch('address')))
end
ENV['INBOUND_EMAIL_AUTHSERV_ID'] = 'mx.mail.test'
html = ['<p>Hello <b>there</b></p><script>bad()</script>', '<head><title>Title</title></head><p>Body</p>', '<div>One<br>Two</div><p>Three</p>', '<template>hidden</template><p>visible</p>', '<table><tr><td>A</td><td>B</td></tr></table>', '<div>A<div>B</div>C</div>', '<p>&amp; &lt; &#160; &#x1f642;</p>', '<svg><title>S</title><script>bad</script><text>T</text></svg>', '<p>a\r\nb</p>', '<!--hi-->a<style>bad</style>b', '<textarea><b>literal</b></textarea>', '<div><p>unclosed', '<pre> a  \n b </pre>']
%w[p div li tr h1 blockquote pre span br script style template].product(['<b>x</b>', 'a &amp; b', "x \t\n\n\ny", '<div>nested</div>']).each { |tag, text| html << "<#{tag}>#{text}</#{tag}><p>end</p>" }
html = html.map { |h| {html: h, expected: mailbox.send(:html_to_text, h)} }
JSON.parse(File.read('/tools/corpus/public-email-html.json')).each do |sample|
  html << sample.merge('expected' => mailbox.send(:html_to_text, sample.fetch('html')))
end
messages = []
travel_to Time.utc(2026, 9, 29, 12, 30) do
  ['Kevin', 'A <B> & "C"', 'Renée', "John\tSmith", 'Doe, John', 'A(B)C', 'A[B]C', 'A\\B', 'John "Smith"', '"John Smith"', 'Jöhn, "Doe"', ' Kevin ', 'Kevin  Smith', 'A' * 80, 'Renée ' * 12, "A\u00a0B", 'é' * 40].each do |name|
    user = Struct.new(:name, :email_address).new(name, 'kevin@example.com')
    session = Struct.new(:device_description).new('Chrome on macOS')
    item = Struct.new(:user, :source, :created_at).new(user, session, Time.current)
    [['sign_in', SecurityMailer.new_sign_in_alert(item).message], ['lockout', TwoFactorMailer.lockout_notice(user).message]].each_with_index do |(kind, mail), i|
      mail.date = Time.current
      mail.message_id = "ws10-#{messages.length}@example.com"
      mail.content_type_parameters['boundary'] = "--==_mimepart_ws10_#{messages.length}"
      mail.body.boundary = mail.content_type_parameters['boundary']
      messages << {kind: kind, name: name, email: user.email_address, device: session.device_description, timestamp: Time.current.iso8601, from: mail[:from].decoded, to: mail[:to].decoded, subject: mail.subject, text: mail.text_part.body.decoded, html: mail.html_part.body.decoded, message_id: mail.message_id, boundary: mail.content_type_parameters['boundary'], raw: mail.encoded}
    end
  end
end
require 'net/smtp'
cram = [['tim', 'tanstaaftanstaaf', '<1896.697170952@postoffice.reston.mci.net>'], ['fixture-user', 'fixture-password', 'relay challenge'], ['fixture-user', 'x' * 80, 'long key challenge']].map do |user, password, challenge|
  digest = Net::SMTP::AuthCramMD5.allocate.send(:cram_md5_response, password, challenge)
  {user: user, password: password, challenge: challenge, response: Base64.strict_encode64("#{user} #{digest}")}
end
basic = ActionController::HttpAuthentication::Basic
credentials = {user: 'actionmailbox', password: 'fixture-mail-password', scheme: 'Basic', separator: ' '}
shapes = [{missing: true}, {literal_chunks: ['']}, {scheme: 'Basic', separator: '', encoded_chunks: []}, credentials,
          credentials.merge(scheme: 'basic'), credentials.merge(scheme: 'basic'), credentials.merge(separator: "\t"), credentials.merge(prefix: ' ', separator: '  '),
          credentials.merge(transform: 'remove_padding'), credentials.merge(transform: 'append_bang'), credentials.merge(transform: 'remove_padding_append_a'), credentials.merge(transform: 'insert_bangs'),
          credentials.merge(scheme: 'Bearer'), {scheme: 'Basic', separator: ' ', encoded_chunks: ['inva', 'lid']}, credentials.merge(user: 'wrong-user'), credentials.merge(password: 'wrong-password')]
whitespace.each { |sample| shapes << credentials.merge(separator: sample.fetch(:value)) }
relay = shapes.map do |shape|
  authorization = if shape[:missing]
    nil
  elsif shape[:literal_chunks]
    shape.fetch(:literal_chunks).join
  else
    token = shape[:encoded_chunks] ? shape.fetch(:encoded_chunks).join : Base64.strict_encode64("#{shape.fetch(:user)}:#{shape.fetch(:password)}")
    token = case shape[:transform]
    when 'remove_padding' then token.delete('=')
    when 'append_bang' then "#{token}!"
    when 'remove_padding_append_a' then "#{token.delete('=')}A"
    when 'insert_bangs' then "#{token[0, 8]}!!#{token[8..]}"
    else token
    end
    "#{shape.fetch(:prefix, '')}#{shape.fetch(:scheme)}#{shape.fetch(:separator)}#{token}"
  end
  request = Struct.new(:authorization).new(authorization)
  expected = !!basic.authenticate(request) { |user, password| user.to_s == 'actionmailbox' && password.to_s == 'fixture-mail-password' }
  shape.merge(expected: expected)
end
review = {deep_mime_raw: Ws10ReviewFixtures.nested_mail(2000), deep_fixed_width_raw: Ws10ReviewFixtures.nested_mail(4000, fixed_width: true), wide: Ws10ReviewFixtures.wide_mail, replay_raw: Ws10ReviewFixtures.replay_mail, retry_raw: Ws10ReviewFixtures.retry_mail}
File.write('/out/reference.json', JSON.pretty_generate({auth: auth, html: html, messages: messages, cram: cram, relay: relay, whitespace: whitespace, review: review}) + "\n")
puts "mail reference: #{auth.size} authentication headers, #{html.size} HTML cases, #{messages.size} MIME messages"
