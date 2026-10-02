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
cases=inputs.map do |name,value|
 sql="UPDATE users SET tour_completed_at=#{conn.quote(value)} WHERE id=127326141"
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
 known_difference=responses.any? { |r| r[:body][/data-tour-auto-start-value="(.*?)"/,1] != main_auto_start }
 {name:,value:,sql:,responses:,known_difference:known_difference ? 'Known main datetime-cast difference; fix after #196 merges' : nil}
end
puts JSON.pretty_generate(reference:'d7c7de92',cases:)
warn "Rails tour datetime corpus: #{cases.size} stored values; #{cases.size*2} HTTP responses"
