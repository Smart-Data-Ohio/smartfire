require 'action_dispatch/testing/integration'
require 'json'
Rails.logger = ActiveSupport::Logger.new($stderr)
Rails.logger.level = Logger::ERROR
ActiveRecord::Base.logger = nil
ApplicationController.allow_forgery_protection = false
labels=JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'parity/.seed/default/labels.json')))
conn=ActiveRecord::Base.connection
inputs={
 'whitespace'=>' \t\n',
 'padded_valid'=>' 2026-01-01 00:00:00 ',
 'long_leading_spaces'=>(' '*129)+'2026-01-01 00:00:00',
 'long_trailing_spaces'=>'2026-01-01 00:00:00'+(' '*129),
 'month_and_year'=>'January 2026',
 'RFC_2822'=>'Fri, 02 Oct 2026 12:00:00 GMT',
 'offset'=>'2026-01-01T23:59:59+05:45',
 'fractional'=>'2026-01-01 00:00:00.123456789',
 'ISO_week_date'=>'2026-W01-1',
 'ordinal_date'=>'2026-001',
 'short_year'=>'26-01-01',
 'year_one'=>'0001-01-01',
 'year_10000'=>'10000-01-01',
 'BC_date'=>'-0044-03-15',
 'invalid_month'=>'2026-13-01',
 'invalid_day'=>'2026-01-32',
 'midnight_24'=>'2026-01-01 24:00:00',
 'leap_second'=>'2026-01-01 23:59:60',
 'date_slashes'=>'2026/10/02',
 'US_date'=>'10/02/2026',
 'named_month'=>'2 October 2026',
 'time_only'=>'12:34:56',
 'month_day_only'=>'October 2',
 'unix_decimal'=>1.5,
 'negative_integer'=>-1,
 'explicit_null'=>nil,
 'very_long_invalid'=>'x'*256,
 'datetime_suffix_junk'=>'2026-01-01 00:00:00 not-a-date',
 'datetime_prefix_junk'=>'not-a-date 2026-01-01 00:00:00'
}
inputs.merge!({
 'year_month_name'=>'2026 Jan',
 'year_month_full'=>'2026 January',
 'month_year_hyphen'=>'Jan-2026',
 'ISO_year_month'=>'2026-01',
 'slash_year_month'=>'2026/01',
 'year_first_words'=>'2026 Jan 1',
 'month_abbrev_dot'=>'Jan. 1, 2026',
 'day_month_abbrev_dot'=>'1 Jan. 2026',
 'month_day_AD_year'=>'January 1, AD 2026',
 'asctime'=>'Thu Jan 1 00:00:00 2026',
 'ISO_without_seconds'=>'2026-01-01T00:00',
 'invalid_hour'=>'2026-01-01 99:00',
 'invalid_minute'=>'2026-01-01 09:99',
 'year_1900_leap_day'=>'1900-02-29',
 'unsigned_extended_year'=>'+2026-01-01',
 'eastern_zone'=>'Thu, 1 Jan 2026 09:00:00 EST'
})
# Preserve the original eight controls and cases, followed by Astra's 45 inputs.
inputs={
 'original_null'=>nil,
 'original_valid_datetime'=>'2026-01-01 00:00:00',
 'original_empty'=>'',
 'original_invalid'=>'not-a-date',
 'original_string_zero'=>'0',
 'original_integer_zero'=>0,
 'original_normalized_day'=>'2026-02-31 12:00:00',
 'original_date'=>'2026-01-01'
}.merge(inputs)
# Additional review-209 values, then BLOB variants of every non-null input.
inputs.merge!({
 'null_byte'=>"2026-01-01\0", 'empty_spaces'=>'   ', 'ascii_whitespace'=>"\t\n\r ",
 'nonbreaking_spaces'=>"\u00a0", 'padded_tabs'=>"\t2026-01-01\n", 'thin_space'=>"2026\u2009Jan\u20091",
 'fraction_12'=>'2026-01-01T12:34:56.123456789012Z', 'fraction_zero'=>'2026-01-01 00:00:00.0000001',
 'fraction_comma'=>'2026-01-01 12:34:56,123456', 'offset_seconds'=>'2026-01-01T12:34:56+05:30:45',
 'offset_negative'=>'2026-01-01T12:34:56-03:30', 'offset_max'=>'2026-01-01T12:34:56+23:59',
 'offset_24'=>'2026-01-01T12:34:56+24:00', 'offset_fraction'=>'2026-01-01 12:34:56 +5.5',
 'two_digit_00'=>'00-01-02', 'two_digit_38'=>'38-12-31', 'two_digit_68'=>'68-12-31',
 'two_digit_69'=>'69-01-01', 'two_digit_99'=>'99-12-31', 'quoted_year'=>"'26", 'year_only'=>'2026',
 'feb_30'=>'2024-02-30', 'feb_29_nonleap'=>'2025-02-29', 'zero_month'=>'2026-00-01',
 'zero_day'=>'2026-01-00', 'negative_month'=>'2026--01-02', 'hour_25'=>'2026-01-01 25:00:00',
 'minute_60'=>'2026-01-01 12:60:00', 'second_61'=>'2026-01-01 12:00:61',
 'hour_24_fraction'=>'2026-01-01 24:00:00.5', 'year_min'=>'-9999-01-01', 'year_max'=>'9999-12-31',
 'huge_year'=>'12345678901234567890-01-01', 'date_128'=>'2026-01-01'+(' '*118),
 'date_129'=>'2026-01-01'+(' '*119), 'float_zero'=>0.0, 'negative_float'=>-12.5,
 'compact_date'=>'20261002', 'week_year'=>'2026-W53-7',
 'unicode_prefix'=>"é2026 Jan 1", 'unicode_suffix'=>"Jan 1 2026é"
})
specs=inputs.map do |name,value|
 quoted=value.is_a?(String) && value.include?("\0") ? "CAST(X'#{value.unpack1('H*')}' AS TEXT)" : conn.quote(value)
 {name:,value:,sql:"UPDATE users SET tour_completed_at=#{quoted} WHERE id=127326141"}
end
inputs.each do |name,value|
 next if value.nil?
 blob=value.is_a?(String) ? "X'#{value.unpack1('H*')}'" : "CAST(#{conn.quote(value)} AS BLOB)"
 specs << {name:"blob_#{name}",value:nil,sql:"UPDATE users SET tour_completed_at=#{blob} WHERE id=127326141"}
end
{
 'valid'=>"2026-01-01 00:00:00", 'named'=>"Jan. 1, 2026", 'empty'=>"", 'invalid'=>"not-a-date",
 'invalid_utf8_only'=>"\xff".b, 'invalid_utf8_prefix'=>"\xff2026-01-01".b,
 'invalid_utf8_suffix'=>"Jan. 1, 2026\xff".b, 'invalid_utf8_middle'=>"2026\xffJan\xff1".b,
 'invalid_utf8_nul'=>"2026-01-01\0\xff".b, 'invalid_utf8_overlong'=>"2026-01-01\xc0\xaf".b,
 'invalid_utf8_truncated'=>"2026-01-01\xe2\x82".b,
 'invalid_utf8_128'=>"2026-01-01".b+("\xff".b*118),
 'invalid_utf8_129'=>"2026-01-01".b+("\xff".b*119)
}.each do |name,bytes|
 specs << {name:"blob_#{name}",value:nil,sql:"UPDATE users SET tour_completed_at=X'#{bytes.unpack1('H*')}' WHERE id=127326141"}
end
cases=specs.map do |spec|
 name,value,sql=spec.values_at(:name,:value,:sql)
 conn.execute(sql)
 responses=['/agents','/users/me/profile'].map do |path|
  b=ActionDispatch::Integration::Session.new(Rails.application);b.host! 'campfire.test'
  b.get(path,headers:{'Cookie'=>"session_token=#{labels.fetch('session_cookies.david')}",'Accept'=>'text/html','HTTP_USER_AGENT'=>'Mozilla/5.0 Chrome/140.0.0.0'})
  fragment=b.response.body[/<div id="tour" hidden.*?^<\/div>/m]
  raise "no tour #{name}: #{b.response.status}" unless fragment
  out={path:,status:b.response.status,body:fragment}
  ActiveSupport::ExecutionContext.clear
  out
 end
 main_auto_start=value.nil? ? 'true' : 'false'
 known_difference=!name.start_with?('blob_') && responses.any? { |r| r[:body][/data-tour-auto-start-value="(.*?)"/,1] != main_auto_start }
 {name:,value:,sql:,responses:,known_difference:known_difference ? 'Known main datetime-cast difference; fix after #196 merges' : nil}
end
puts JSON.pretty_generate(reference:'d7c7de92',cases:)
warn "Rails tour datetime corpus: #{cases.size} stored values; #{cases.size*2} HTTP responses"
