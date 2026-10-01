# Capture Ruby's Unicode semantics and the application's actual writers/matcher.
# No normalization is applied to the outputs; transactions only isolate fixture cases.
require "rbconfig"
raise "Ruby runtime drift" unless RUBY_REVISION == "2b0b7728dc7f0561c35c3d8c4489945c94b783ad"
ActiveRecord::Base.logger = nil
user = User.find_by!(email_address: "david@37signals.com")
maps = %i[downcase fold upcase].to_h { |operation| [operation, []] }
word_ranges = []
word_start = nil
alpha_ranges = []
alpha_start = nil
(0..0x10ffff).each do |point|
  next if (0xd800..0xdfff).cover?(point)
  char = [point].pack("U")
  maps.each do |operation, rows|
    value = operation == :fold ? char.downcase(:fold) : char.public_send(operation)
    rows << [point, value] if value != char
  end
  if /[[:alpha:]]/.match?(char)
    alpha_start ||= point
  elsif alpha_start
    alpha_ranges << [alpha_start, point - 1]
    alpha_start = nil
  end
  word = /[\p{L}\p{N}_]/.match?(char)
  if word
    word_start ||= point
  elsif word_start
    word_ranges << [word_start, point - 1]
    word_start = nil
  end
end
word_ranges << [word_start, 0x10ffff] if word_start
alpha_ranges << [alpha_start, 0x10ffff] if alpha_start
# All nontrivial full folds, both directions, including surrounding word boundaries
# and combining marks. The explicit cross product probes unequal folds as well.
strings = ["straße", "STRASSE", "ß", "ẞ", "s", "ss", "ﬀ", "ﬃ", "ffi", "ff", "office", "oﬃce", "οσ", "ος", "ΟΣ", "σ", "ς", "Σ", "I", "i", "İ", "ı", "i̇", "é", "é", "ǰ", "ǰ", "j", "ʼn", "ŉ", "ᾀ", "ἀι", "Ꭰ", "ꭰ", "deploy failed", "v1.2 (rc)", "", " ", "\u00a0"]
pairs = strings.product(strings)
maps[:fold].each do |point, folded|
  char = [point].pack("U")
  pairs.concat([[char, folded], [folded, char], [char, "x#{folded}"], [char, "#{folded}x"], [char, "(#{folded})"], [char, "#{folded}\u0301"], [char, "\u0301#{folded}"]])
end
pairs.concat([["s", "ß s"], ["s", "s ß"], ["s", "ß foo s"], ["i", "İ i"], ["i", "i İ"], ["i", "İ foo i"], ["ff", "ﬃ ff"], ["f", "ﬀ f"], ["j", "ǰ j"], ["ⱥ", "Ⱥ "], ["ⱦ", "Ⱦ!"], ["deploy failed", "deploy\nfailed"], ["deploy failed", "deploy\u00a0failed"], ["ś", "ß́"], ["s", "ß's"], ["i", "İ i"], ["ffi", "ﬃffi"], ["deploy  failed", "deploy failed"]])
matcher = pairs.uniq.map { |phrase, text| {phrase:, text:, users: Notifications::KeywordMatcher.matching_user_ids({17 => [phrase]}, text)} }
replacements = ["οσ\nΟΣ", "ος\nΟΣ", "ΟΣ\nοσ", "ß\nss\nẞ", "ﬃ\nffi", "I\ni\nİ\ni̇\nı", "σ\nς\nΣ", "é\né", "ŉ\nʼn"].map do |lines|
  row = nil
  User.transaction(requires_new: true) do
    user.reload.errors.clear
    success = user.replace_keyword_alerts(lines)
    row = {lines:, success:, phrases: user.keyword_alerts.order(:id).pluck(:phrase), errors: user.errors.full_messages}
    raise ActiveRecord::Rollback
  end
  row
end
validations = [["οσ", "ΟΣ"], ["ος", "ΟΣ"], ["ΟΣ", "οσ"], ["ß", "ẞ"], ["ss", "ß"], ["i̇", "İ"], ["ﬃ", "ffi"]].map do |existing, candidate|
  row = nil
  KeywordAlert.transaction(requires_new: true) do
    user.keyword_alerts.delete_all
    user.keyword_alerts.create!(phrase: existing)
    alert = user.keyword_alerts.new(phrase: candidate)
    row = {existing:, candidate:, valid: alert.valid?, errors: alert.errors.full_messages}
    raise ActiveRecord::Rollback
  end
  row
end
humanized = ["ΟΣ", "straße", "ﬃ_ready", "_ready_id", "user_id_id", " HTTP_oauth_STATUS ", "1_ΟΣ", "ß_status", "{ΟΣ}_ID"].map { |value| {value:, humanized: value.humanize} }
tags = ["ΟΣ", "οσ", "ος", " STRASSE ", "straße", " Σ "]
thread = ChannelThread.new
thread.tag_names = tags
normalized_tags = thread.tag_names
comparison_pairs = ["ΟΣ", "οσ", "ος", "STRASSE", "straße", "ﬃ", "ffi", "İ", "i̇", "ı", "I", "é", "é"].product(["ΟΣ", "οσ", "ος", "STRASSE", "straße", "ﬃ", "ffi", "İ", "i̇", "ı", "I", "é", "é"])
comparison_pairs.concat([["\u1c89@example.test", "\u1c8a@example.test"], ["\ua7cb@example.test", "\u0264@example.test"], ["\ua7cc@example.test", "\ua7cd@example.test"], ["\u{10d50}@example.test", "\u{10d70}@example.test"], [" Σ@example.test ", "ς@example.test"]])
comparisons = comparison_pairs.map do |left, right|
  controller = Users::ProfilesController.new
  controller.params = ActionController::Parameters.new(user: {email_address: right})
  controller.instance_variable_set(:@user, User.new(email_address: left))
  {left:, right:, equal: left.casecmp?(right), email_changing: controller.send(:email_change_requested?)}
end

puts JSON.generate(reference: "d7c7de92", ruby: RUBY_VERSION, unicode: RbConfig::CONFIG.fetch("UNICODE_VERSION"), maps:, word_ranges:, alpha_ranges:, matcher:, replacements:, validations:, humanized:, tags:, normalized_tags:, comparisons:)
